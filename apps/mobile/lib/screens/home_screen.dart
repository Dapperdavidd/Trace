import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../core/copy.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';
import '../widgets/tiles.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> {
  int _tab = 0;
  static const _tabs = ['For You', 'Top Movers', 'Smart Traders'];

  @override
  Widget build(BuildContext context) {
    return TraceScaffold(
      title: 'TRACE',
      actions: [
        IconButton(
          tooltip: 'Alerts',
          icon: const Icon(Icons.notifications_none),
          onPressed: () => context.push('/alerts'),
        ),
      ],
      body: ListView(
        padding: const EdgeInsets.all(TraceSpacing.gutter),
        children: [
          const _WatchlistCard(),
          const _PaperCard(),
          const ComingLaterPanel(
            title: 'Send, Receive, Buy, Swap',
            explanation:
                'TRACE does not hold funds yet. Wallet actions arrive in a later version.',
            icon: Icons.account_balance_wallet_outlined,
          ),
          TraceCard(
            onTap: () => context.push('/briefing'),
            semanticsLabel: 'Daily briefing. Your market summary.',
            child: const Row(children: [
              Icon(Icons.wb_sunny_outlined, color: TraceColors.emerald),
              SizedBox(width: 12),
              Expanded(
                  child: Text('Daily briefing',
                      style: TextStyle(fontWeight: FontWeight.w700))),
              Icon(Icons.chevron_right),
            ]),
          ),
          const SectionTitle("What's happening"),
          Wrap(
            spacing: 8,
            children: [
              for (var i = 0; i < _tabs.length; i++)
                ChoiceChip(
                  label: Text(_tabs[i]),
                  selected: _tab == i,
                  onSelected: (_) => setState(() => _tab = i),
                ),
            ],
          ),
          const SizedBox(height: 12),
          if (_tab == 2)
            const _SmartTraders()
          else
            _Movers(limit: _tab == 0 ? 3 : 5),
          const Disclaimer(),
        ],
      ),
    );
  }
}

class _Movers extends ConsumerWidget {
  const _Movers({required this.limit});
  final int limit;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    const q = ('trending', '24h', '');
    final v = ref.watch(moversProvider(q));
    return v.when(
      loading: () => const Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: CircularProgressIndicator())),
      error: (e, _) => ErrorState(
          error: e, onRetry: () => ref.invalidate(moversProvider(q))),
      data: (r) => Column(children: [
        Align(alignment: Alignment.centerLeft, child: EnvelopeBar(r.env)),
        if (r.items.isEmpty)
          const EmptyState(
              title: 'Nothing moving yet',
              message: 'No tokens passed the liquidity and activity filters.'),
        for (final m in r.items.take(limit)) MoverTile(m),
      ]),
    );
  }
}

class _SmartTraders extends ConsumerWidget {
  const _SmartTraders();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(discoveryWalletsProvider('top'));
    return v.when(
      loading: () => const Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: CircularProgressIndicator())),
      error: (e, _) => ErrorState(
          error: e,
          onRetry: () => ref.invalidate(discoveryWalletsProvider('top'))),
      data: (r) => Column(children: [
        Align(alignment: Alignment.centerLeft, child: EnvelopeBar(r.env)),
        if (r.items.isEmpty)
          const EmptyState(
              title: 'No wallets yet',
              message: 'No wallet has enough history to show.'),
        for (final w in r.items) WalletTile(w),
      ]),
    );
  }
}

class _WatchlistCard extends ConsumerWidget {
  const _WatchlistCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(watchlistProvider);
    final text = v.when(
      loading: () => 'Loading your watchlist...',
      error: (_, __) => 'Watchlist unavailable right now.',
      data: (items) {
        if (items.isEmpty) {
          return 'No wallets watched yet. Add any public address.';
        }
        final attention =
            items.where((i) => i.status != WatchStatus.active).length;
        return '${items.length} wallet${items.length == 1 ? '' : 's'} watched'
            '${attention > 0 ? ', $attention not fully monitored' : ''}.';
      },
    );
    return TraceCard(
      onTap: () => context.go('/watchlist'),
      semanticsLabel: 'Your watchlist. $text',
      child: Row(children: [
        const Icon(Icons.visibility_outlined, color: TraceColors.emerald),
        const SizedBox(width: 12),
        Expanded(
          child:
              Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            const Text('Your watchlist',
                style: TextStyle(fontWeight: FontWeight.w700, fontSize: 16)),
            Muted(text),
            const Muted(
                'Watch-only. No balances are shown because TRACE holds no funds.',
                size: 12),
          ]),
        ),
        const Icon(Icons.chevron_right),
      ]),
    );
  }
}

class _PaperCard extends ConsumerWidget {
  const _PaperCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final ids = ref.watch(shadowIdsProvider);
    if (ids.isEmpty) {
      return TraceCard(
        onTap: () => context.go('/shadow'),
        semanticsLabel: 'Paper results. No simulation yet. Start one.',
        child: const Row(children: [
          Icon(Icons.science_outlined, color: TraceColors.emerald),
          SizedBox(width: 12),
          Expanded(
            child:
                Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text('Paper results',
                  style: TextStyle(fontWeight: FontWeight.w700, fontSize: 16)),
              Muted(
                  'No simulation yet. See what would have happened if you shadowed a trader. No real money.'),
            ]),
          ),
          Icon(Icons.chevron_right),
        ]),
      );
    }
    final v = ref.watch(shadowResultsProvider(ids.first));
    return v.when(
      loading: () => const TraceCard(child: Text('Loading paper results...')),
      error: (_, __) =>
          const TraceCard(child: Text('Paper results unavailable right now.')),
      data: (r) => TraceCard(
        onTap: () => context.push('/shadow/results/${ids.first}'),
        semanticsLabel:
            'Paper results. Simulated return ${signedPercentString(r.returnPct)}. Simulation, not real money.',
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          const Text('Paper results',
              style: TextStyle(fontWeight: FontWeight.w700, fontSize: 16)),
          Text(signedPercentString(r.returnPct),
              style: TextStyle(
                  fontSize: 28,
                  fontWeight: FontWeight.w800,
                  color: changeColor(parseDecimal(r.returnPct)))),
          const Muted('Simulated result, not real money.', size: 12),
          const Muted(Copy.paperOnly, size: 12),
        ]),
      ),
    );
  }
}
