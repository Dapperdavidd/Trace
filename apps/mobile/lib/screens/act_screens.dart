import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:http/http.dart' as http;
import 'package:url_launcher/url_launcher.dart';

import '../api/models.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../state/session.dart';
import '../wallet/near_core.dart';
import '../wallet/wallet_state.dart';
import '../widgets/common.dart';

const EdgeInsets _pad = EdgeInsets.fromLTRB(16, 8, 16, 120);

String _short(String a) {
  if (RegExp(r'^[a-z0-9_.-]+\.(near|testnet)$').hasMatch(a) && a.length <= 30) {
    return a;
  }
  return a.length > 12 ? '${a.substring(0, 4)}…${a.substring(a.length - 3)}' : a;
}

String _money(double v) =>
    '\$${v.toStringAsFixed(2).replaceAllMapped(RegExp(r'\B(?=(\d{3})+(?!\d))'), (_) => ',')}';

void _toast(BuildContext context, String m) => ScaffoldMessenger.of(context)
  ..hideCurrentSnackBar()
  ..showSnackBar(SnackBar(content: Text(m)));

Future<void> _open(String url) =>
    launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);

class _Panel extends StatelessWidget {
  const _Panel({required this.children, this.label});
  final String? label;
  final List<Widget> children;
  @override
  Widget build(BuildContext context) => Container(
        width: double.infinity,
        margin: const EdgeInsets.only(bottom: 12),
        padding: const EdgeInsets.all(18),
        decoration: BoxDecoration(
            color: TraceColors.surface,
            borderRadius: BorderRadius.circular(TraceSpacing.radius)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (label != null) ...[
              Text(label!,
                  style: const TextStyle(color: TraceColors.textFaint, fontSize: 13)),
              const SizedBox(height: 10),
            ],
            ...children,
          ],
        ),
      );
}

class _NetSwitch extends ConsumerWidget {
  const _NetSwitch();
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(walletProvider);
    return SegmentedButton<NearNet>(
      showSelectedIcon: false,
      segments: const [
        ButtonSegment(value: NearNet.mainnet, label: Text('Mainnet')),
        ButtonSegment(value: NearNet.testnet, label: Text('Testnet')),
      ],
      selected: {s.network},
      onSelectionChanged: (v) => ref.read(walletProvider.notifier).setNetwork(v.first),
    );
  }
}

/* =============================== Wallet =============================== */

class WalletScreen extends ConsumerStatefulWidget {
  const WalletScreen({super.key});
  @override
  ConsumerState<WalletScreen> createState() => _WalletScreenState();
}

class _WalletScreenState extends ConsumerState<WalletScreen> {
  bool busy = false;
  String? error;
  String panel = 'none';
  int tab = 0;
  final to = TextEditingController();
  final amount = TextEditingController();
  final secret = TextEditingController();
  final importId = TextEditingController();

  @override
  void dispose() {
    for (final c in [to, amount, secret, importId]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> run(Future<void> Function() fn) async {
    setState(() {
      busy = true;
      error = null;
    });
    try {
      await fn();
    } catch (e) {
      setState(() => error = e.toString().replaceFirst('Exception: ', ''));
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  void refresh() {
    ref.invalidate(walletBalanceProvider);
    ref.invalidate(walletActivityProvider);
  }

  @override
  Widget build(BuildContext context) {
    final s = ref.watch(walletProvider);
    final w = s.current;
    final ctl = ref.read(walletProvider.notifier);
    final session = ref.watch(sessionProvider);
    final hint = session.name ?? session.email?.split('@').first ?? 'trace';

    if (!s.loaded) {
      return const TraceScaffold(
          title: 'Wallet', body: Center(child: CircularProgressIndicator()));
    }

    if (w == null) {
      final testnet = s.network == NearNet.testnet;
      return TraceScaffold(
        title: 'Wallet',
        subtitle: 'Your keys stay on this phone',
        body: ListView(padding: _pad, children: [
          const _NetSwitch(),
          const SizedBox(height: 14),
          _Panel(label: testnet ? 'Testnet wallet' : 'Mainnet wallet', children: [
            Text(
                testnet
                    ? 'Try everything with free test NEAR.'
                    : 'Your real NEAR wallet.',
                style: const TextStyle(
                    fontSize: 22, fontWeight: FontWeight.w500, letterSpacing: -0.6)),
            const SizedBox(height: 8),
            Muted(
                testnet
                    ? 'trace creates a testnet account and asks the NEAR faucet for 10 test NEAR. Test tokens have no value.'
                    : 'trace creates a NEAR account from a key generated on this phone. It starts empty; send NEAR to it to trade.',
                size: 14),
            const SizedBox(height: 16),
            SizedBox(
              width: double.infinity,
              child: FilledButton(
                onPressed: busy
                    ? null
                    : () => run(() async {
                          await ctl.create(s.network, hint);
                          refresh();
                          if (context.mounted) {
                            _toast(context,
                                testnet ? 'Testnet wallet ready with 10 test NEAR' : 'Mainnet wallet created');
                          }
                        }),
                child: Text(busy
                    ? (testnet ? 'Asking the faucet' : 'Creating')
                    : testnet
                        ? 'Create testnet wallet'
                        : 'Create mainnet wallet'),
              ),
            ),
            const SizedBox(height: 8),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton(
                onPressed: () => setState(() => panel = panel == 'import' ? 'none' : 'import'),
                child: const Text('I have a NEAR key'),
              ),
            ),
            if (panel == 'import') ...[
              const SizedBox(height: 12),
              TextField(
                  controller: secret,
                  decoration: const InputDecoration(hintText: 'Secret key (ed25519:…)')),
              const SizedBox(height: 8),
              TextField(
                  controller: importId,
                  decoration: InputDecoration(
                      hintText: testnet ? 'name.testnet (optional)' : 'name.near (optional)')),
              const SizedBox(height: 8),
              SizedBox(
                width: double.infinity,
                child: FilledButton(
                  onPressed: busy
                      ? null
                      : () => run(() async {
                            await ctl.import(s.network, secret.text, importId.text);
                            secret.clear();
                            refresh();
                          }),
                  child: const Text('Restore wallet'),
                ),
              ),
            ],
            if (error != null) ...[
              const SizedBox(height: 10),
              Text(error!, style: const TextStyle(color: TraceColors.textMuted)),
            ],
          ]),
        ]),
      );
    }

    final balance = ref.watch(walletBalanceProvider);
    final price = ref.watch(nearPriceProvider).valueOrNull;
    final bal = balance.valueOrNull;
    final near = bal == null ? 0.0 : double.parse(yoctoToNear(bal, dp: 6).replaceAll(',', ''));
    return TraceScaffold(
      title: 'Wallet',
      subtitle: '${s.network.label} · ${_short(w.accountId)}',
      actions: [
        IconButton(
          tooltip: 'Copy address',
          icon: const Icon(Icons.copy_rounded, size: 20),
          onPressed: () {
            Clipboard.setData(ClipboardData(text: w.accountId));
            _toast(context, 'Address copied');
          },
        ),
      ],
      body: RefreshIndicator(
        onRefresh: () async => refresh(),
        child: ListView(padding: _pad, children: [
          const _NetSwitch(),
          const SizedBox(height: 18),
          Text(balance.isLoading ? '…' : '${yoctoToNear(bal ?? BigInt.zero, dp: 2)} NEAR',
              style: const TextStyle(
                  fontSize: 44, fontWeight: FontWeight.w500, letterSpacing: -2, height: 1)),
          const SizedBox(height: 6),
          Text(
              price == null || bal == null
                  ? (bal == null && !balance.isLoading ? 'Not on chain yet' : ' ')
                  : _money(near * price),
              style: const TextStyle(color: TraceColors.textMuted)),
          const SizedBox(height: 16),
          Row(children: [
            _Action(Icons.north_east, 'Send', () => setState(() => panel = panel == 'send' ? 'none' : 'send')),
            _Action(Icons.south_west, 'Receive', () => setState(() => panel = panel == 'receive' ? 'none' : 'receive')),
            _Action(Icons.swap_horiz, 'Swap', () => context.go('/trade')),
            s.network == NearNet.testnet
                ? _Action(Icons.add, busy ? 'Asking' : s.faucetCooling ? 'Sent' : 'Get tokens',
                    busy || s.faucetCooling
                        ? null
                        : () => run(() async {
                              await ctl.requestTokens();
                              if (context.mounted) _toast(context, '9.95 test NEAR is on its way');
                              await Future<void>.delayed(const Duration(seconds: 3));
                              refresh();
                            }),
                    primary: true)
                : _Action(Icons.add, 'Buy', () => context.go('/trade'), primary: true),
          ]),
          if (error != null)
            Padding(
                padding: const EdgeInsets.only(top: 12),
                child: Text(error!, style: const TextStyle(color: TraceColors.textMuted))),
          const SizedBox(height: 14),
          if (panel == 'send')
            _Panel(label: 'Send NEAR', children: [
              TextField(
                  controller: to,
                  autocorrect: false,
                  decoration: InputDecoration(
                      hintText: s.network == NearNet.testnet ? 'friend.testnet' : 'friend.near')),
              const SizedBox(height: 8),
              TextField(
                  controller: amount,
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(hintText: 'Amount in NEAR')),
              const SizedBox(height: 12),
              SizedBox(
                width: double.infinity,
                child: FilledButton(
                  onPressed: busy
                      ? null
                      : () async {
                          final ok = await showDialog<bool>(
                              context: context,
                              builder: (c) => AlertDialog(
                                    title: const Text('Send NEAR?'),
                                    content: Text(
                                        'Send ${amount.text} NEAR to ${to.text} on ${s.network.label.toLowerCase()}. This cannot be undone.'),
                                    actions: [
                                      TextButton(onPressed: () => Navigator.pop(c, false), child: const Text('Cancel')),
                                      FilledButton(onPressed: () => Navigator.pop(c, true), child: const Text('Send')),
                                    ],
                                  ));
                          if (ok != true) return;
                          await run(() async {
                            await ctl.send(to.text, amount.text);
                            if (context.mounted) _toast(context, 'Sent ${amount.text} NEAR');
                            to.clear();
                            amount.clear();
                            setState(() => panel = 'none');
                            await Future<void>.delayed(const Duration(seconds: 2));
                            refresh();
                          });
                        },
                  child: Text(busy ? 'Sending' : 'Send'),
                ),
              ),
            ]),
          if (panel == 'receive')
            _Panel(label: 'Your ${s.network.label.toLowerCase()} address', children: [
              SelectableText(w.accountId,
                  style: const TextStyle(fontSize: 15, fontFamily: 'Courier')),
              const SizedBox(height: 12),
              Row(children: [
                FilledButton(
                    onPressed: () {
                      Clipboard.setData(ClipboardData(text: w.accountId));
                      _toast(context, 'Address copied');
                    },
                    child: const Text('Copy')),
                const SizedBox(width: 8),
                OutlinedButton(
                    onPressed: () => _open('${s.network.explorer}/address/${w.accountId}'),
                    child: const Text('NearBlocks')),
              ]),
            ]),
          SegmentedButton<int>(
            showSelectedIcon: false,
            segments: const [
              ButtonSegment(value: 0, label: Text('Tokens')),
              ButtonSegment(value: 1, label: Text('Activity')),
            ],
            selected: {tab},
            onSelectionChanged: (v) => setState(() => tab = v.first),
          ),
          const SizedBox(height: 10),
          if (tab == 0)
            ListTile(
              leading: const TokenAvatar('NEAR'),
              title: const Text('NEAR'),
              subtitle: Text('${yoctoToNear(bal ?? BigInt.zero)} NEAR'),
              trailing: Text(price == null ? '-' : _money(near * price)),
            )
          else
            ref.watch(walletActivityProvider).when(
                  loading: () => const Padding(
                      padding: EdgeInsets.all(12), child: Muted('Loading activity…')),
                  error: (_, __) => const Muted('Activity could not load.'),
                  data: (items) => items.isEmpty
                      ? const Padding(padding: EdgeInsets.all(12), child: Muted('No transactions yet.'))
                      : Column(children: [
                          for (final a in items)
                            ListTile(
                              onTap: () => _open('${s.network.explorer}/txns/${a.hash}'),
                              leading: Icon(a.incoming ? Icons.south_west : Icons.north_east,
                                  color: a.incoming ? TraceColors.emerald : TraceColors.textMuted),
                              title: Text(
                                  '${a.incoming ? 'Received from' : a.kind == 'transfer' ? 'Sent to' : a.kind} ${_short(a.counterparty)}'),
                              subtitle: Text(
                                  '${a.time.toLocal().toString().substring(0, 16)}${a.ok ? '' : ' · failed'}'),
                              trailing: Text(a.amount > BigInt.zero
                                  ? '${a.incoming ? '+' : '−'}${yoctoToNear(a.amount, dp: 3)}'
                                  : ''),
                            ),
                        ]),
                ),
          const SizedBox(height: 16),
          _Panel(label: 'Backup', children: [
            const Muted(
                'Your key lives only on this phone. Save it somewhere safe; without it nobody can recover this wallet.',
                size: 13.5),
            if (panel == 'secret') ...[
              const SizedBox(height: 10),
              SelectableText(w.secretKey, style: const TextStyle(fontSize: 12)),
            ],
            const SizedBox(height: 10),
            Row(children: [
              OutlinedButton(
                onPressed: () async {
                  if (panel == 'secret') return setState(() => panel = 'none');
                  final ok = await showDialog<bool>(
                      context: context,
                      builder: (c) => AlertDialog(
                            title: const Text('Show secret key?'),
                            content: const Text('Anyone who sees it can take everything in this wallet.'),
                            actions: [
                              TextButton(onPressed: () => Navigator.pop(c, false), child: const Text('Cancel')),
                              FilledButton(onPressed: () => Navigator.pop(c, true), child: const Text('Show')),
                            ],
                          ));
                  if (ok == true) setState(() => panel = 'secret');
                },
                child: Text(panel == 'secret' ? 'Hide key' : 'Show secret key'),
              ),
              const Spacer(),
              TextButton(
                onPressed: () async {
                  final ok = await showDialog<bool>(
                      context: context,
                      builder: (c) => AlertDialog(
                            title: const Text('Remove this wallet?'),
                            content: const Text('Without your secret key backup it is gone for good.'),
                            actions: [
                              TextButton(onPressed: () => Navigator.pop(c, false), child: const Text('Cancel')),
                              FilledButton(onPressed: () => Navigator.pop(c, true), child: const Text('Remove')),
                            ],
                          ));
                  if (ok == true) await ctl.forget(s.network);
                },
                child: const Text('Remove'),
              ),
            ]),
          ]),
        ]),
      ),
    );
  }
}

class _Action extends StatelessWidget {
  const _Action(this.icon, this.label, this.onTap, {this.primary = false});
  final IconData icon;
  final String label;
  final VoidCallback? onTap;
  final bool primary;
  @override
  Widget build(BuildContext context) => Expanded(
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 3),
          child: Material(
            color: primary ? TraceColors.emerald : TraceColors.surface,
            borderRadius: BorderRadius.circular(18),
            child: InkWell(
              borderRadius: BorderRadius.circular(18),
              onTap: onTap,
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 14),
                child: Column(children: [
                  Icon(icon, size: 20, color: primary ? TraceColors.onEmerald : TraceColors.text),
                  const SizedBox(height: 6),
                  Text(label,
                      style: TextStyle(
                          fontSize: 12.5,
                          fontWeight: FontWeight.w500,
                          color: primary ? TraceColors.onEmerald : TraceColors.text)),
                ]),
              ),
            ),
          ),
        ),
      );
}

/* =============================== Trade (NEAR Intents) =============================== */

class _Asset {
  _Asset(this.id, this.symbol, this.chain, this.decimals, this.price);
  final String id;
  final String symbol;
  final String chain;
  final int decimals;
  final double? price;
  String get label => symbol == 'wNEAR' ? 'NEAR' : symbol;
}

const _picks = [
  ['near', 'wNEAR'],
  ['near', 'USDC'],
  ['near', 'USDT'],
  ['eth', 'ETH'],
  ['sol', 'SOL'],
  ['btc', 'BTC'],
];
const _chainLabel = {'near': 'NEAR', 'eth': 'Ethereum', 'sol': 'Solana', 'btc': 'Bitcoin'};

final _intentAssets = FutureProvider<List<_Asset>>((ref) async {
  final res = await http
      .get(Uri.parse('https://1click.chaindefuser.com/v0/tokens'))
      .timeout(const Duration(seconds: 20));
  final all = (jsonDecode(res.body) as List).cast<Map>();
  return [
    for (final p in _picks)
      for (final a in all.where((a) => a['blockchain'] == p[0] && a['symbol'] == p[1]).take(1))
        _Asset(a['assetId'] as String, a['symbol'] as String, a['blockchain'] as String,
            a['decimals'] as int, (a['price'] as num?)?.toDouble()),
  ];
});

String? _baseUnits(String amount, int decimals) {
  final m = RegExp(r'^(\d*)(?:\.(\d*))?$').firstMatch(amount.trim());
  if (m == null || ((m.group(1) ?? '').isEmpty && (m.group(2) ?? '').isEmpty)) return null;
  final frac = (m.group(2) ?? '').padRight(decimals, '0').substring(0, decimals);
  final s = ((m.group(1) ?? '').isEmpty ? '0' : m.group(1)!) + frac;
  final t = s.replaceFirst(RegExp(r'^0+'), '');
  return t.isEmpty ? null : t;
}

class TradeScreen extends ConsumerStatefulWidget {
  const TradeScreen({super.key});
  @override
  ConsumerState<TradeScreen> createState() => _TradeScreenState();
}

class _TradeScreenState extends ConsumerState<TradeScreen> {
  int from = 0;
  int to = 1;
  final amount = TextEditingController(text: '1');
  Map<String, dynamic>? quote;
  bool busy = false;
  String? error;

  @override
  void dispose() {
    amount.dispose();
    super.dispose();
  }

  Future<void> getQuote(List<_Asset> assets, String account) async {
    final a = assets[from], b = assets[to];
    final base = _baseUnits(amount.text, a.decimals);
    if (base == null) return setState(() => error = 'Type an amount, for example 1 or 0.5.');
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final res = await http.post(Uri.parse('https://1click.chaindefuser.com/v0/quote'),
          headers: {'Content-Type': 'application/json'},
          body: jsonEncode({
            'dry': true,
            'swapType': 'EXACT_INPUT',
            'slippageTolerance': 100,
            'originAsset': a.id,
            'depositType': 'INTENTS',
            'destinationAsset': b.id,
            'amount': base,
            'recipient': account,
            'recipientType': 'INTENTS',
            'refundTo': account,
            'refundType': 'INTENTS',
            'deadline': DateTime.now().toUtc().add(const Duration(minutes: 10)).toIso8601String(),
          }));
      final body = jsonDecode(res.body) as Map<String, dynamic>;
      if (res.statusCode >= 300 || body['quote'] == null) {
        throw Exception('NEAR Intents: ${body['message'] ?? 'no quote for this pair'}');
      }
      setState(() => quote = (body['quote'] as Map).cast<String, dynamic>());
    } catch (e) {
      setState(() => error = e.toString().replaceFirst('Exception: ', ''));
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final s = ref.watch(walletProvider);
    final mainnet = s.mainnet;
    final assets = ref.watch(_intentAssets);
    return TraceScaffold(
      title: 'Trade',
      subtitle: 'Live quotes from NEAR Intents',
      body: assets.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (_, __) => const EmptyState(
            title: 'NEAR Intents is not reachable', message: 'Try again in a moment.'),
        data: (list) {
          if (list.length < 2) {
            return const EmptyState(title: 'No assets', message: 'NEAR Intents returned no assets.');
          }
          final a = list[from], b = list[to];
          final q = quote;
          final usdIn = a.price != null ? (double.tryParse(amount.text) ?? 0) * a.price! : null;
          return ListView(padding: _pad, children: [
            if (s.network == NearNet.testnet)
              const _Panel(children: [
                Text('NEAR Intents only runs on mainnet. Quotes here are real mainnet prices.'),
              ]),
            _Panel(label: 'You pay', children: [
              Row(children: [
                Expanded(
                  child: TextField(
                    controller: amount,
                    keyboardType: const TextInputType.numberWithOptions(decimal: true),
                    style: const TextStyle(fontSize: 30, fontWeight: FontWeight.w500),
                    decoration: const InputDecoration(
                        filled: false, contentPadding: EdgeInsets.zero, border: InputBorder.none),
                    onChanged: (_) => setState(() => quote = null),
                  ),
                ),
                ActionChip(
                  label: Text('${a.label} ▾'),
                  onPressed: () => setState(() {
                    from = (from + 1) % list.length;
                    if (from == to) from = (from + 1) % list.length;
                    quote = null;
                  }),
                ),
              ]),
              Muted(q != null
                  ? '\$${double.parse(q['amountInUsd'] as String).toStringAsFixed(2)}'
                  : usdIn != null
                      ? 'about ${_money(usdIn)}'
                      : ''),
            ]),
            Center(
              child: IconButton(
                icon: const Icon(Icons.swap_vert),
                onPressed: () => setState(() {
                  final t = from;
                  from = to;
                  to = t;
                  quote = null;
                }),
              ),
            ),
            _Panel(label: 'You get', children: [
              Row(children: [
                Expanded(
                  child: Text(
                      q == null
                          ? '-'
                          : double.parse(q['amountOutFormatted'] as String)
                              .toStringAsFixed(6)
                              .replaceFirst(RegExp(r'\.?0+$'), ''),
                      style: const TextStyle(fontSize: 30, fontWeight: FontWeight.w500)),
                ),
                ActionChip(
                  label: Text('${b.label} ▾'),
                  onPressed: () => setState(() {
                    to = (to + 1) % list.length;
                    if (to == from) to = (to + 1) % list.length;
                    quote = null;
                  }),
                ),
              ]),
              Muted(q != null
                  ? '\$${double.parse(q['amountOutUsd'] as String).toStringAsFixed(2)} · on ${_chainLabel[b.chain] ?? b.chain} · ~${q['timeEstimate']}s'
                  : b.price != null
                      ? '1 ${b.label} ≈ ${_money(b.price!)}'
                      : ''),
            ]),
            if (error != null)
              Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: Text(error!, style: const TextStyle(color: TraceColors.textMuted))),
            SizedBox(
              width: double.infinity,
              height: 52,
              child: mainnet == null
                  ? FilledButton(
                      onPressed: () async {
                        await ref.read(walletProvider.notifier).setNetwork(NearNet.mainnet);
                        if (context.mounted) context.go('/wallet');
                      },
                      child: const Text('Create mainnet wallet'))
                  : FilledButton(
                      onPressed: busy
                          ? null
                          : () => q != null
                              ? _toast(context,
                                  'Fund your mainnet wallet with NEAR to place this swap. Your wallet signs it; trace never holds funds.')
                              : getQuote(list, mainnet.accountId),
                      child: Text(busy ? 'Getting a live quote' : q != null ? 'Confirm swap' : 'Get live quote'),
                    ),
            ),
            const SizedBox(height: 10),
            const Muted('Quotes settle into your NEAR account. Your wallet signs every swap.', size: 12.5),
          ]);
        },
      ),
    );
  }
}

/* =============================== Ask trace =============================== */

class AskScreen extends ConsumerStatefulWidget {
  const AskScreen({super.key});
  @override
  ConsumerState<AskScreen> createState() => _AskScreenState();
}

class _Answer {
  _Answer(this.q, this.text, {this.coins = const [], this.wallets = const []});
  final String q;
  final String text;
  final List<MoverItem> coins;
  final List<WalletSummary> wallets;
}

class _AskScreenState extends ConsumerState<AskScreen> {
  final input = TextEditingController();
  final thread = <_Answer>[];

  @override
  void dispose() {
    input.dispose();
    super.dispose();
  }

  _Answer answer(String q, List<MoverItem> coins, List<WalletSummary> wallets) {
    final up = ' ${q.toUpperCase().replaceAll(RegExp(r'[^A-Z0-9 ]'), ' ')} ';
    MoverItem? c;
    for (final m in coins) {
      final sym = (m.token.symbol ?? '').toUpperCase().replaceAll(RegExp(r'[^A-Z0-9]'), '');
      if (sym.length > 1 && up.contains(' $sym ')) {
        c = m;
        break;
      }
    }
    if (coins.isEmpty && wallets.isEmpty) {
      return _Answer(q, 'trace has no live coins or ranked wallets to answer from yet. Try the daily briefing.');
    }
    if (c != null && RegExp('SAFE|RISK|SCAM|RUG').hasMatch(up)) {
      return _Answer(q, '${c.token.displayName} risk: ${c.risk.name}. Open it for each check.', coins: [c]);
    }
    if (c != null) {
      return _Answer(q, '${c.token.displayName} is ${c.changePct ?? '?'}% now. ${c.reasonLabel}.', coins: [c]);
    }
    if (RegExp('WALLET|SMART|COPY|TRADER').hasMatch(up)) {
      return wallets.isEmpty
          ? _Answer(q, 'No wallets have enough evidence to be ranked yet.')
          : _Answer(q, 'These are the top ranked wallets right now. Test one with pretend money before you copy it.',
              wallets: wallets.take(4).toList());
    }
    return coins.isEmpty
        ? _Answer(q, 'No coins have cleared the filters yet. Ask about wallets instead.')
        : _Answer(q, 'Here is what is moving most right now.', coins: coins.take(3).toList());
  }

  @override
  Widget build(BuildContext context) {
    final movers = ref.watch(moversProvider(('trending', '24h', ''))).valueOrNull?.items ?? const <MoverItem>[];
    final hour = movers.isEmpty
        ? ref.watch(moversProvider(('trending', '1h', ''))).valueOrNull?.items ?? const <MoverItem>[]
        : const <MoverItem>[];
    final coins = movers.isNotEmpty ? movers : hour;
    final wallets = ref.watch(discoveryWalletsProvider('top')).valueOrNull?.items ?? const <WalletSummary>[];
    void ask(String q) {
      if (q.trim().isEmpty) return;
      setState(() => thread.add(answer(q.trim(), coins, wallets)));
      input.clear();
    }

    const suggestions = ['What is moving right now?', 'Which smart wallets should I look at?'];
    return TraceScaffold(
      title: 'Ask trace',
      subtitle: 'Answers come from live coins and wallets',
      body: Column(children: [
        Expanded(
          child: ListView(padding: const EdgeInsets.fromLTRB(16, 8, 16, 16), children: [
            if (thread.isEmpty) const Muted('Ask about any coin or wallet trace can see.', size: 14),
            for (final t in thread) ...[
              Align(
                alignment: Alignment.centerRight,
                child: Container(
                  margin: const EdgeInsets.symmetric(vertical: 8),
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                  decoration: BoxDecoration(color: TraceColors.text, borderRadius: BorderRadius.circular(18)),
                  child: Text(t.q, style: const TextStyle(color: TraceColors.bg)),
                ),
              ),
              Text(t.text),
              for (final m in t.coins)
                ListTile(
                  leading: TokenAvatar(m.token.displayName),
                  title: Text(m.token.displayName),
                  subtitle: Text(m.reasonLabel),
                  trailing: Text('${m.changePct ?? ''}%'),
                  onTap: () => context.push(m.token.path),
                ),
              for (final w in t.wallets)
                ListTile(
                  leading: const Icon(Icons.account_circle_outlined),
                  title: Text(w.wallet.displayName),
                  subtitle: Text('${w.wallet.chain.label} · ${w.winRatePct == null ? '${w.sampleSize} trades' : '${w.winRatePct}% win'}'),
                  onTap: () => context.push(w.wallet.path),
                ),
            ],
            const SizedBox(height: 12),
            Wrap(spacing: 8, runSpacing: 8, children: [
              for (final sgg in suggestions) ActionChip(label: Text(sgg), onPressed: () => ask(sgg)),
            ]),
          ]),
        ),
        SafeArea(
          top: false,
          minimum: const EdgeInsets.fromLTRB(16, 0, 16, 100),
          child: Row(children: [
            Expanded(
              child: TextField(
                controller: input,
                textInputAction: TextInputAction.send,
                onSubmitted: ask,
                decoration: const InputDecoration(hintText: 'Ask about any coin or wallet'),
              ),
            ),
            const SizedBox(width: 8),
            IconButton.filled(
              style: IconButton.styleFrom(backgroundColor: TraceColors.emerald, foregroundColor: TraceColors.onEmerald),
              icon: const Icon(Icons.arrow_forward),
              onPressed: () => ask(input.text),
            ),
          ]),
        ),
      ]),
    );
  }
}
