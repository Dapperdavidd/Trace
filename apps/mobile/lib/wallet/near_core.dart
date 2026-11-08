/// NEAR wallet core for the trace app: keys, Borsh transaction encoding, signing and RPC.
/// Pure Dart (no Flutter) so it can be exercised from a script. Keys never leave the device;
/// the trace API never sees them.
library;

import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as hashing;
import 'package:ed25519_edwards/ed25519_edwards.dart' as ed;
import 'package:http/http.dart' as http;

enum NearNet { mainnet, testnet }

extension NearNetInfo on NearNet {
  String get rpc => this == NearNet.mainnet
      ? 'https://free.rpc.fastnear.com'
      : 'https://test.rpc.fastnear.com';
  String get nearblocksApi => this == NearNet.mainnet
      ? 'https://api.nearblocks.io'
      : 'https://api-testnet.nearblocks.io';
  String get explorer => this == NearNet.mainnet
      ? 'https://nearblocks.io'
      : 'https://testnet.nearblocks.io';
  String get label => this == NearNet.mainnet ? 'Mainnet' : 'Testnet';
}

const _faucet = 'https://helper.nearprotocol.com/account';
final BigInt yoctoPerNear = BigInt.from(10).pow(24);

/* ---------------- base58 ---------------- */

const _alphabet = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

String base58Encode(List<int> bytes) {
  var n = BigInt.zero;
  for (final b in bytes) {
    n = (n << 8) | BigInt.from(b);
  }
  final out = StringBuffer();
  final b58 = BigInt.from(58);
  while (n > BigInt.zero) {
    out.write(_alphabet[(n % b58).toInt()]);
    n = n ~/ b58;
  }
  for (final b in bytes) {
    if (b != 0) break;
    out.write('1');
  }
  return out.toString().split('').reversed.join();
}

Uint8List base58Decode(String s) {
  var n = BigInt.zero;
  for (final c in s.split('')) {
    final i = _alphabet.indexOf(c);
    if (i < 0) throw const FormatException('not base58');
    n = n * BigInt.from(58) + BigInt.from(i);
  }
  final bytes = <int>[];
  while (n > BigInt.zero) {
    bytes.insert(0, (n & BigInt.from(0xff)).toInt());
    n = n >> 8;
  }
  for (final c in s.split('')) {
    if (c != '1') break;
    bytes.insert(0, 0);
  }
  return Uint8List.fromList(bytes);
}

/* ---------------- keys ---------------- */

class NearKey {
  NearKey(this.secret) : assert(secret.length == 64);

  /// 64 bytes: 32-byte seed followed by the 32-byte public key (NEAR's format).
  final Uint8List secret;

  factory NearKey.random() {
    final kp = ed.generateKey();
    return NearKey(Uint8List.fromList(kp.privateKey.bytes));
  }

  factory NearKey.parse(String s) {
    final raw = s.trim().startsWith('ed25519:') ? s.trim().substring(8) : s.trim();
    final bytes = base58Decode(raw);
    if (bytes.length == 64) return NearKey(bytes);
    if (bytes.length == 32) {
      return NearKey(Uint8List.fromList(ed.newKeyFromSeed(bytes).bytes));
    }
    throw const FormatException('A NEAR secret key is 64 bytes.');
  }

  Uint8List get publicBytes => Uint8List.fromList(secret.sublist(32));
  String get publicKey => 'ed25519:${base58Encode(publicBytes)}';
  String get secretKey => 'ed25519:${base58Encode(secret)}';

  /// The 64-hex implicit account id for this key.
  String get implicitAccount =>
      publicBytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

  Uint8List sign(Uint8List message) =>
      ed.sign(ed.PrivateKey(secret.toList()), message);
}

/* ---------------- Borsh ---------------- */

class _Borsh {
  final BytesBuilder _b = BytesBuilder();
  void u8(int v) => _b.addByte(v);
  void u32(int v) {
    final d = ByteData(4)..setUint32(0, v, Endian.little);
    _b.add(d.buffer.asUint8List());
  }

  void u64(BigInt v) => _uint(v, 8);
  void u128(BigInt v) => _uint(v, 16);
  void _uint(BigInt v, int len) {
    final out = Uint8List(len);
    var x = v;
    for (var i = 0; i < len; i++) {
      out[i] = (x & BigInt.from(0xff)).toInt();
      x = x >> 8;
    }
    _b.add(out);
  }

  void str(String s) {
    final u = utf8.encode(s);
    u32(u.length);
    _b.add(u);
  }

  void fixed(List<int> bytes) => _b.add(bytes);
  Uint8List done() => _b.toBytes();
}

/// A NEAR transaction with Transfer actions, encoded exactly as the protocol expects.
Uint8List encodeTransfer({
  required String signerId,
  required Uint8List publicKey,
  required BigInt nonce,
  required String receiverId,
  required Uint8List blockHash,
  required BigInt deposit,
}) {
  final b = _Borsh()
    ..str(signerId)
    ..u8(0) // ed25519
    ..fixed(publicKey)
    ..u64(nonce)
    ..str(receiverId)
    ..fixed(blockHash)
    ..u32(1) // one action
    ..u8(3) // Action::Transfer
    ..u128(deposit);
  return b.done();
}

Uint8List signTransaction(Uint8List tx, NearKey key) {
  final hash = Uint8List.fromList(hashing.sha256.convert(tx).bytes);
  final sig = key.sign(hash);
  return (BytesBuilder()
        ..add(tx)
        ..addByte(0) // ed25519 signature
        ..add(sig))
      .toBytes();
}

/* ---------------- amounts ---------------- */

BigInt nearToYocto(String amount) {
  final m = RegExp(r'^(\d*)(?:\.(\d*))?$').firstMatch(amount.trim());
  if (m == null || ((m.group(1) ?? '').isEmpty && (m.group(2) ?? '').isEmpty)) {
    return BigInt.zero;
  }
  final whole = (m.group(1) ?? '').isEmpty ? '0' : m.group(1)!;
  final frac = (m.group(2) ?? '').padRight(24, '0').substring(0, 24);
  return BigInt.parse(whole + frac);
}

String yoctoToNear(BigInt y, {int dp = 4}) {
  final whole = y ~/ yoctoPerNear;
  final frac = (y % yoctoPerNear)
      .toString()
      .padLeft(24, '0')
      .substring(0, dp)
      .replaceFirst(RegExp(r'0+$'), '');
  final w = whole.toString().replaceAllMapped(
      RegExp(r'\B(?=(\d{3})+(?!\d))'), (_) => ',');
  return frac.isEmpty ? w : '$w.$frac';
}

/* ---------------- RPC ---------------- */

class NearRpcException implements Exception {
  NearRpcException(this.message);
  final String message;
  @override
  String toString() => message;
}

class NearRpc {
  NearRpc(this.net, {http.Client? client}) : _http = client ?? http.Client();
  final NearNet net;
  final http.Client _http;

  Future<Map<String, dynamic>> call(String method, Object params) async {
    final res = await _http
        .post(Uri.parse(net.rpc),
            headers: {'Content-Type': 'application/json'},
            body: jsonEncode({
              'jsonrpc': '2.0',
              'id': 'trace',
              'method': method,
              'params': params,
            }))
        .timeout(const Duration(seconds: 40));
    final body = jsonDecode(res.body) as Map<String, dynamic>;
    final err = body['error'];
    if (err != null) {
      final cause = (err as Map)['cause'] as Map?;
      throw NearRpcException(
          (cause?['name'] ?? err['message'] ?? 'NEAR RPC error').toString());
    }
    return (body['result'] as Map).cast<String, dynamic>();
  }

  /// Available balance in yocto, or null when the account does not exist yet.
  Future<BigInt?> balance(String accountId) async {
    try {
      final r = await call('query', {
        'request_type': 'view_account',
        'finality': 'final',
        'account_id': accountId,
      });
      final amount = BigInt.parse(r['amount'] as String);
      final storage = BigInt.from(r['storage_usage'] as int) *
          BigInt.parse('10000000000000000000');
      final available = amount - storage;
      return available < BigInt.zero ? BigInt.zero : available;
    } on NearRpcException catch (e) {
      if (e.message.contains('UNKNOWN_ACCOUNT')) return null;
      rethrow;
    }
  }

  Future<String> transfer(
      String signerId, NearKey key, String receiverId, BigInt deposit) async {
    final ak = await call('query', {
      'request_type': 'view_access_key',
      'finality': 'final',
      'account_id': signerId,
      'public_key': key.publicKey,
    });
    final nonce = BigInt.from(ak['nonce'] as int) + BigInt.one;
    final blockHash = base58Decode(ak['block_hash'] as String);
    final tx = encodeTransfer(
      signerId: signerId,
      publicKey: key.publicBytes,
      nonce: nonce,
      receiverId: receiverId,
      blockHash: blockHash,
      deposit: deposit,
    );
    final signed = signTransaction(tx, key);
    final r = await call('send_tx', {
      'signed_tx_base64': base64Encode(signed),
      'wait_until': 'EXECUTED_OPTIMISTIC',
    });
    final status = r['status'];
    if (status is Map && status.containsKey('Failure')) {
      throw NearRpcException('The transfer failed on chain.');
    }
    return ((r['transaction'] as Map?)?['hash'] ?? '') as String;
  }
}

/* ---------------- faucet ---------------- */

String _rand(int n) {
  const a = 'abcdefghijklmnopqrstuvwxyz0123456789';
  final r = Random.secure();
  return List.generate(n, (_) => a[r.nextInt(a.length)]).join();
}

String testnetName(String hint) {
  var s = hint.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]'), '');
  if (s.length > 16) s = s.substring(0, 16);
  if (s.length < 2) s = 'trace';
  return '$s-${_rand(5)}.testnet';
}

Future<void> faucetCreate(String accountId, String publicKey,
    {http.Client? client}) async {
  final c = client ?? http.Client();
  final res = await c
      .post(Uri.parse(_faucet),
          headers: {'Content-Type': 'application/json'},
          body: jsonEncode(
              {'newAccountId': accountId, 'newAccountPublicKey': publicKey}))
      .timeout(const Duration(seconds: 60));
  if (res.statusCode == 429) {
    throw NearRpcException('The NEAR faucet is busy. Try again in a few minutes.');
  }
  if (res.statusCode >= 300) {
    throw NearRpcException('The NEAR faucet refused the request.');
  }
}

/// Testnet tokens: the faucet funds new accounts, so trace asks it for a fresh helper
/// account and moves its NEAR into yours. Returns the transfer hash.
Future<String> requestTestnetTokens(String accountId,
    {http.Client? client}) async {
  final helperKey = NearKey.random();
  final helperId = 'faucet-${_rand(10)}.testnet';
  await faucetCreate(helperId, helperKey.publicKey, client: client);
  final rpc = NearRpc(NearNet.testnet, client: client);
  for (var i = 0; i < 12; i++) {
    if (await rpc.balance(helperId) != null) break;
    await Future<void>.delayed(const Duration(milliseconds: 1200));
  }
  return rpc.transfer(helperId, helperKey, accountId, nearToYocto('9.95'));
}

/* ---------------- activity and price ---------------- */

class NearActivity {
  NearActivity(this.hash, this.time, this.kind, this.counterparty,
      this.incoming, this.amount, this.ok);
  final String hash;
  final DateTime time;
  final String kind;
  final String counterparty;
  final bool incoming;
  final BigInt amount;
  final bool ok;
}

Future<List<NearActivity>> fetchActivity(NearNet net, String accountId,
    {http.Client? client}) async {
  final c = client ?? http.Client();
  final res = await c
      .get(Uri.parse(
          '${net.nearblocksApi}/v3/accounts/${Uri.encodeComponent(accountId)}/txns?per_page=15'))
      .timeout(const Duration(seconds: 20));
  if (res.statusCode != 200) return [];
  final data = (jsonDecode(res.body) as Map)['data'] as List? ?? [];
  return data.map((raw) {
    final t = (raw as Map).cast<String, dynamic>();
    final signer = (t['signer_account_id'] ?? t['predecessor_account_id'] ?? '').toString();
    final receiver = (t['receiver_account_id'] ?? '').toString();
    final incoming = receiver == accountId && signer != accountId;
    final actions = (t['actions'] as List? ?? []).cast<Map>();
    final first = actions.isEmpty ? null : actions.first;
    final kind = first == null
        ? 'Transaction'
        : first['action'] == 'FUNCTION_CALL'
            ? (first['method'] ?? 'Contract call').toString()
            : first['action'].toString().replaceAll('_', ' ').toLowerCase();
    final deposit = ((t['actions_agg'] as Map?)?['deposit'] ?? '0').toString().split('.').first;
    final ts = BigInt.tryParse((t['block_timestamp'] ?? '0').toString()) ?? BigInt.zero;
    return NearActivity(
      (t['transaction_hash'] ?? '').toString(),
      DateTime.fromMillisecondsSinceEpoch((ts ~/ BigInt.from(1000000)).toInt()),
      kind,
      incoming ? signer : receiver,
      incoming,
      BigInt.tryParse(deposit) ?? BigInt.zero,
      (t['outcomes'] as Map?)?['status'] != false,
    );
  }).toList();
}

Future<double?> nearUsdPrice({http.Client? client}) async {
  try {
    final c = client ?? http.Client();
    final res = await c
        .get(Uri.parse('https://coins.llama.fi/prices/current/coingecko:near'))
        .timeout(const Duration(seconds: 15));
    final p = (((jsonDecode(res.body) as Map)['coins'] as Map?)?['coingecko:near']
        as Map?)?['price'];
    return p is num ? p.toDouble() : null;
  } catch (_) {
    return null;
  }
}
