import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'near_core.dart';

/// One stored wallet. The secret key lives only in the device's secure storage.
class StoredWallet {
  const StoredWallet(
      {required this.accountId,
      required this.publicKey,
      required this.secretKey,
      required this.createdAt});
  final String accountId;
  final String publicKey;
  final String secretKey;
  final DateTime createdAt;

  Map<String, dynamic> toJson() => {
        'accountId': accountId,
        'publicKey': publicKey,
        'secretKey': secretKey,
        'createdAt': createdAt.toIso8601String(),
      };
  static StoredWallet? fromJson(Object? j) {
    if (j is! Map) return null;
    return StoredWallet(
      accountId: j['accountId'] as String,
      publicKey: j['publicKey'] as String,
      secretKey: j['secretKey'] as String,
      createdAt: DateTime.tryParse(j['createdAt'] as String? ?? '') ??
          DateTime.now(),
    );
  }

  NearKey get key => NearKey.parse(secretKey);
}

class WalletState {
  const WalletState(
      {this.loaded = false,
      this.network = NearNet.testnet,
      this.testnet,
      this.mainnet,
      this.lastFaucetAt});
  final bool loaded;
  final NearNet network;
  final StoredWallet? testnet;
  final StoredWallet? mainnet;
  final DateTime? lastFaucetAt;

  StoredWallet? get current => network == NearNet.mainnet ? mainnet : testnet;
  bool get faucetCooling =>
      lastFaucetAt != null &&
      DateTime.now().difference(lastFaucetAt!) < const Duration(minutes: 10);

  WalletState copy(
          {NearNet? network,
          StoredWallet? testnet,
          StoredWallet? mainnet,
          DateTime? lastFaucetAt,
          bool clearTestnet = false,
          bool clearMainnet = false}) =>
      WalletState(
        loaded: true,
        network: network ?? this.network,
        testnet: clearTestnet ? null : testnet ?? this.testnet,
        mainnet: clearMainnet ? null : mainnet ?? this.mainnet,
        lastFaucetAt: lastFaucetAt ?? this.lastFaucetAt,
      );
}

const _storeKey = 'trace.wallet.v1';

class WalletController extends Notifier<WalletState> {
  final FlutterSecureStorage _store = const FlutterSecureStorage();

  @override
  WalletState build() {
    Future<void>.microtask(_load);
    return const WalletState();
  }

  Future<void> _load() async {
    try {
      final raw = await _store.read(key: _storeKey);
      if (raw == null) {
        state = const WalletState(loaded: true);
        return;
      }
      final j = jsonDecode(raw) as Map<String, dynamic>;
      state = WalletState(
        loaded: true,
        network: j['network'] == 'mainnet' ? NearNet.mainnet : NearNet.testnet,
        testnet: StoredWallet.fromJson(j['testnet']),
        mainnet: StoredWallet.fromJson(j['mainnet']),
        lastFaucetAt: j['lastFaucetAt'] is int
            ? DateTime.fromMillisecondsSinceEpoch(j['lastFaucetAt'] as int)
            : null,
      );
    } catch (_) {
      state = const WalletState(loaded: true);
    }
  }

  Future<void> _save() async {
    try {
      await _store.write(
          key: _storeKey,
          value: jsonEncode({
            'network': state.network.name,
            if (state.testnet != null) 'testnet': state.testnet!.toJson(),
            if (state.mainnet != null) 'mainnet': state.mainnet!.toJson(),
            if (state.lastFaucetAt != null)
              'lastFaucetAt': state.lastFaucetAt!.millisecondsSinceEpoch,
          }));
    } catch (_) {
      // storage failure: the wallet lives until the app closes
    }
  }

  Future<void> setNetwork(NearNet n) async {
    state = state.copy(network: n);
    await _save();
  }

  /// Create this network's wallet. Testnet accounts are created and funded by the faucet.
  Future<StoredWallet> create(NearNet net, String nameHint) async {
    final existing = net == NearNet.mainnet ? state.mainnet : state.testnet;
    if (existing != null) return existing;
    final key = NearKey.random();
    final id = net == NearNet.testnet ? testnetName(nameHint) : key.implicitAccount;
    if (net == NearNet.testnet) await faucetCreate(id, key.publicKey);
    final w = StoredWallet(
        accountId: id,
        publicKey: key.publicKey,
        secretKey: key.secretKey,
        createdAt: DateTime.now());
    state = net == NearNet.mainnet
        ? state.copy(mainnet: w)
        : state.copy(testnet: w, lastFaucetAt: DateTime.now());
    await _save();
    return w;
  }

  Future<void> import(NearNet net, String secret, String accountId) async {
    final key = NearKey.parse(secret);
    final id = accountId.trim().isEmpty ? key.implicitAccount : accountId.trim();
    final w = StoredWallet(
        accountId: id,
        publicKey: key.publicKey,
        secretKey: key.secretKey,
        createdAt: DateTime.now());
    state = net == NearNet.mainnet ? state.copy(mainnet: w) : state.copy(testnet: w);
    await _save();
  }

  Future<String> requestTokens() async {
    final w = state.testnet;
    if (w == null) throw NearRpcException('Create a testnet wallet first.');
    if (state.faucetCooling) {
      throw NearRpcException('You just got testnet tokens. Try again in a few minutes.');
    }
    final hash = await requestTestnetTokens(w.accountId);
    state = state.copy(lastFaucetAt: DateTime.now());
    await _save();
    return hash;
  }

  Future<String> send(String to, String amount) async {
    final w = state.current;
    if (w == null) throw NearRpcException('No wallet on this network.');
    final yocto = nearToYocto(amount);
    if (yocto <= BigInt.zero) throw NearRpcException('Type an amount above zero.');
    return NearRpc(state.network).transfer(w.accountId, w.key, to.trim(), yocto);
  }

  Future<void> forget(NearNet net) async {
    state = net == NearNet.mainnet
        ? state.copy(clearMainnet: true)
        : state.copy(clearTestnet: true);
    await _save();
  }
}

final walletProvider =
    NotifierProvider<WalletController, WalletState>(WalletController.new);

/// Live balance of the current wallet (null when the account is not on chain yet).
final walletBalanceProvider = FutureProvider.autoDispose<BigInt?>((ref) async {
  final s = ref.watch(walletProvider);
  final w = s.current;
  if (w == null) return null;
  return NearRpc(s.network).balance(w.accountId);
});

final nearPriceProvider =
    FutureProvider.autoDispose<double?>((ref) => nearUsdPrice());

final walletActivityProvider =
    FutureProvider.autoDispose<List<NearActivity>>((ref) async {
  final s = ref.watch(walletProvider);
  final w = s.current;
  if (w == null) return [];
  return fetchActivity(s.network, w.accountId);
});
