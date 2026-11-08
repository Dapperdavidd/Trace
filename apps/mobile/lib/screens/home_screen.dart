import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

const MoversQuery _homeMovers = ('trending', '24h', '');

/// The deliberately focused Explore surface: movement, wallet accumulation,
/// and the top three tokens. Detail and filters live one level deeper.
class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> {
  Timer? _refresh;

  @override
  void initState() {
    super.initState();
    _refresh = Timer.periodic(const Duration(seconds: 30), (_) {
      if (mounted) ref.invalidate(moversProvider(_homeMovers));
    });
  }

  @override
  void dispose() {
    _refresh?.cancel();
    super.dispose();
  }

  Future<void> _reload() async {
    ref.invalidate(moversProvider(_homeMovers));
    await ref.read(moversProvider(_homeMovers).future);
  }

  @override
  Widget build(BuildContext context) {
    final movers = ref.watch(moversProvider(_homeMovers));
    final demo = ref.watch(demoActiveProvider);

    return Scaffold(
      backgroundColor: TraceColors.bg,
      body: SafeArea(
        bottom: false,
        child: Column(
          children: [
            const _ExploreHeader(),
            ValueListenableBuilder<bool>(
              valueListenable: demo,
              builder: (_, active, __) =>
                  active ? const DemoBanner() : const SizedBox.shrink(),
            ),
            Expanded(
              child: RefreshIndicator(
                onRefresh: _reload,
                child: movers.when(
                  loading: () => const _LoadingExplore(),
                  error: (error, _) => _UnavailableExplore(
                    error: error,
                    onRetry: _reload,
                  ),
                  data: (response) => _ExploreResults(response),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ExploreHeader extends StatelessWidget {
  const _ExploreHeader();

  @override
  Widget build(BuildContext context) => SizedBox(
        width: double.infinity,
        height: 78,
        child: Stack(
          alignment: Alignment.center,
          children: [
            Positioned(
              left: 18,
              child: IconButton(
                tooltip: 'Your profile',
                onPressed: () => context.go('/you'),
                icon: const Icon(Icons.person_rounded, size: 21),
              ),
            ),
            Semantics(
              header: true,
              label: 'trace',
              child: const ExcludeSemantics(
                child: Text.rich(
                  TextSpan(children: [
                    TextSpan(text: 'trace'),
                    TextSpan(
                      text: '.',
                      style: TextStyle(color: TraceColors.emerald),
                    ),
                  ]),
                  style: TextStyle(
                    color: TraceColors.text,
                    fontSize: 30,
                    fontWeight: FontWeight.w600,
                    letterSpacing: -1.7,
                  ),
                ),
              ),
            ),
            Positioned(
              right: 18,
              child: IconButton(
                tooltip: 'Search movers',
                onPressed: () => context.push('/movers'),
                style: IconButton.styleFrom(
                  backgroundColor: Colors.transparent,
                  foregroundColor: TraceColors.text,
                ),
                icon: const Icon(Icons.search_rounded, size: 26),
              ),
            ),
          ],
        ),
      );
}

class _ExploreResults extends StatelessWidget {
  const _ExploreResults(this.response);
  final MoversResponse response;

  @override
  Widget build(BuildContext context) {
    final items = response.items.take(10).toList();
    final moverCount = items.isEmpty ? kNotAvailable : '${items.length}';
    final accumulating = response.items
        .where((item) =>
            item.reasonCode == 'smart_wallets_buying' ||
            item.reasonCode == 'whale_accumulation' ||
            item.reasonCode == 'accumulation')
        .toList();

    return ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(20, 42, 20, 116),
      children: [
        _HeroCount(count: moverCount),
        const SizedBox(height: 44),
        _AccumulationCard(
          item: accumulating.isEmpty ? null : accumulating.first,
          status: response.env.status,
        ),
        const SizedBox(height: 34),
        _MoversHeader(env: response.env),
        const SizedBox(height: 12),
        if (response.items.isEmpty)
          const _NoMoversCard()
        else
          _TopMoversCard(items: response.items.take(3).toList()),
      ],
    );
  }
}

class _LoadingExplore extends StatelessWidget {
  const _LoadingExplore();

  @override
  Widget build(BuildContext context) => ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(20, 72, 20, 116),
        children: const [
          _HeroCount(count: '…'),
          SizedBox(height: 64),
          Center(child: CircularProgressIndicator()),
        ],
      );
}

class _UnavailableExplore extends StatelessWidget {
  const _UnavailableExplore({required this.error, required this.onRetry});
  final Object error;
  final Future<void> Function() onRetry;

  @override
  Widget build(BuildContext context) => ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(20, 42, 20, 116),
        children: [
          const _HeroCount(count: kNotAvailable),
          const SizedBox(height: 40),
          ErrorState(error: error, onRetry: onRetry),
        ],
      );
}

class _HeroCount extends StatelessWidget {
  const _HeroCount({required this.count});
  final String count;

  @override
  Widget build(BuildContext context) => Semantics(
        label: 'Onchain right now. $count coins moving. Ethereum and Solana.',
        child: ExcludeSemantics(
          child: Column(
            children: [
              const Text(
                'Onchain right now',
                style: TextStyle(
                  color: TraceColors.textMuted,
                  fontSize: 17,
                  fontWeight: FontWeight.w400,
                ),
              ),
              const SizedBox(height: 10),
              Text(
                count,
                style: TextStyle(
                  color: TraceColors.text,
                  fontSize: count == kNotAvailable ? 31 : 64,
                  fontWeight: FontWeight.w500,
                  height: 1,
                  letterSpacing: count == kNotAvailable ? -1 : -3,
                ),
              ),
              const SizedBox(height: 8),
              const Text(
                'coins moving',
                style: TextStyle(
                  color: TraceColors.textMuted,
                  fontSize: 20,
                  fontWeight: FontWeight.w600,
                  letterSpacing: -.5,
                ),
              ),
              const SizedBox(height: 20),
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 18, vertical: 10),
                decoration: BoxDecoration(
                  color: TraceColors.emeraldDeep,
                  borderRadius: BorderRadius.circular(999),
                ),
                child: const Text(
                  'Ethereum + Solana',
                  style: TextStyle(
                    color: TraceColors.emerald,
                    fontSize: 15,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
            ],
          ),
        ),
      );
}

class _AccumulationCard extends StatelessWidget {
  const _AccumulationCard({
    required this.item,
    required this.status,
  });
  final MoverItem? item;
  final DataStatus status;

  @override
  Widget build(BuildContext context) {
    final available = item != null;
    final title = available
        ? 'Tracked wallets are\naccumulating.'
        : 'Wallet accumulation\nunavailable.';
    final message = available
        ? 'Multiple reviewed wallets are moving into ${item!.token.displayName}. Review the activity and risk before acting.'
        : 'Current wallet flows do not have enough verified liquidity data yet.';
    return Material(
      color: TraceColors.surface,
      borderRadius: BorderRadius.circular(22),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: available ? () => context.push(item!.token.path) : null,
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                width: 46,
                height: 46,
                decoration: BoxDecoration(
                  color: TraceColors.emeraldDeep,
                  borderRadius: BorderRadius.circular(14),
                ),
                child: const Icon(Icons.bar_chart_rounded,
                    color: TraceColors.emerald),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title,
                        style: const TextStyle(
                          color: TraceColors.text,
                          fontSize: 20,
                          fontWeight: FontWeight.w600,
                          height: 1.08,
                          letterSpacing: -.5,
                        )),
                    const SizedBox(height: 10),
                    Text(message,
                        style: const TextStyle(
                          color: TraceColors.textMuted,
                          fontSize: 13.5,
                          height: 1.4,
                        )),
                    const SizedBox(height: 10),
                    _CompactStatus(status: status),
                  ],
                ),
              ),
              if (available) ...[
                const SizedBox(width: 8),
                const Padding(
                  padding: EdgeInsets.only(top: 28),
                  child: Icon(Icons.chevron_right_rounded,
                      color: TraceColors.textMuted),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _MoversHeader extends StatelessWidget {
  const _MoversHeader({required this.env});
  final Envelope env;

  @override
  Widget build(BuildContext context) => Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Top movers',
                    style: TextStyle(
                      color: TraceColors.text,
                      fontSize: 21,
                      fontWeight: FontWeight.w600,
                      letterSpacing: -.5,
                    )),
                const SizedBox(height: 5),
                Row(children: [
                  _CompactStatus(status: env.status),
                  const SizedBox(width: 7),
                  Text('Updated ${relativeTime(env.asOf)}',
                      style: const TextStyle(
                          color: TraceColors.textFaint, fontSize: 11)),
                ]),
              ],
            ),
          ),
          TextButton(
            onPressed: () => context.push('/movers'),
            child: const Row(mainAxisSize: MainAxisSize.min, children: [
              Text('View all'),
              SizedBox(width: 4),
              Icon(Icons.arrow_forward_rounded, size: 18),
            ]),
          ),
        ],
      );
}

class _TopMoversCard extends StatelessWidget {
  const _TopMoversCard({required this.items});
  final List<MoverItem> items;

  @override
  Widget build(BuildContext context) => Container(
        decoration: BoxDecoration(
          color: TraceColors.surface,
          borderRadius: BorderRadius.circular(22),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          children: [
            for (var i = 0; i < items.length; i++) ...[
              _CompactMoverRow(item: items[i]),
              if (i != items.length - 1) const Divider(height: 1, indent: 72),
            ],
          ],
        ),
      );
}

class _CompactMoverRow extends StatelessWidget {
  const _CompactMoverRow({required this.item});
  final MoverItem item;

  @override
  Widget build(BuildContext context) {
    final change = parseDecimal(item.changePct);
    final secondary = item.token.name?.trim().isNotEmpty == true
        ? item.token.name!
        : item.token.chain.label;
    return Semantics(
      button: true,
      label:
          '${item.token.displayName}, $secondary, ${signedPercent(change, decimals: 2)}',
      child: InkWell(
        onTap: () => context.push(item.token.path),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          child: Row(
            children: [
              TokenAvatar(item.token.displayName, size: 42),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(item.token.displayName,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          fontSize: 17,
                          fontWeight: FontWeight.w600,
                          letterSpacing: -.2,
                        )),
                    const SizedBox(height: 2),
                    Text(secondary,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                            color: TraceColors.textMuted, fontSize: 13)),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              Text(
                signedPercent(change, decimals: 2),
                style: TextStyle(
                  color: changeColor(change),
                  fontSize: 17,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CompactStatus extends StatelessWidget {
  const _CompactStatus({required this.status});
  final DataStatus status;

  @override
  Widget build(BuildContext context) {
    final (label, color) = switch (status) {
      DataStatus.live => ('Live', TraceColors.positive),
      DataStatus.stale => ('Stale', TraceColors.warning),
      DataStatus.partial => ('Partial', TraceColors.warning),
      DataStatus.demo => ('Demo', TraceColors.warning),
      DataStatus.unavailable => ('Unavailable', TraceColors.textMuted),
    };
    return Row(mainAxisSize: MainAxisSize.min, children: [
      Container(
          width: 6,
          height: 6,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle)),
      const SizedBox(width: 5),
      Text(label,
          style: TextStyle(
              color: color, fontSize: 11, fontWeight: FontWeight.w500)),
    ]);
  }
}

class _NoMoversCard extends StatelessWidget {
  const _NoMoversCard();

  @override
  Widget build(BuildContext context) => Container(
        padding: const EdgeInsets.all(22),
        decoration: BoxDecoration(
          color: TraceColors.surface,
          borderRadius: BorderRadius.circular(22),
        ),
        child: const Column(
          children: [
            Text('Movers unavailable',
                style: TextStyle(fontSize: 17, fontWeight: FontWeight.w600)),
            SizedBox(height: 6),
            Text(
              'The market worker has not produced current snapshots yet. Pull down to check again.',
              textAlign: TextAlign.center,
              style: TextStyle(color: TraceColors.textMuted, height: 1.4),
            ),
          ],
        ),
      );
}
