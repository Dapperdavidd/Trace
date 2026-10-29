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

class WalletProfileScreen extends ConsumerWidget {
  const WalletProfileScreen({super.key, required this.chain, required this.address, this.summary});
  final Chain chain;
  final String address;

  /// Discovery row (tags, win rate) when the user came from a list.
  final WalletSummary? summary;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final key = (chain, address);
    final detail = ref.watch(walletDetailProvider(key));
    final title = detail.valueOrNull?.wallet.displayName ?? shortAddress(address);
    return DefaultTabController(
      length: 3,
      child: TraceScaffold(
        title: title,
        subtitle: '${chain.label} wallet (public address)',
        bottom: const TabBar(tabs: [Tab(text: 'Overview'), Tab(text: 'Trades'), Tab(text: 'Performance')]),
        body: AsyncBody<WalletDetail>(
          value: detail,
          onRetry: () => ref.invalidate(walletDetailProvider(key)),
          builder: (d) => Column(children: [
            Expanded(
              child: TabBarView(children: [
                _Overview(d, summary, key),
                _Trades(key),
                _Performance(key),
              ]),
            ),
            SafeArea(
              top: false,
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Row(children: [
                  Expanded(
                    child: OutlinedButton(
                      onPressed: () async {
                        final messenger = ScaffoldMessenger.of(context);
                        try {
                          await ref.read(apiProvider).addWatch(chain, address);
                          ref.invalidate(watchlistProvider);
                          messenger.showSnackBar(const SnackBar(content: Text('Added to your watchlist.')));
                        } catch (_) {
                          messenger.showSnackBar(const SnackBar(content: Text('Could not add to watchlist.')));
                        }
                      },
                      child: const Text('Watch'),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    flex: 2,
                    child: FilledButton(
                      onPressed: () => context.push('/shadow/setup?chain=${chain.wire}&address=$address'),
                      child: const Text('Shadow this wallet'),
                    ),
                  ),
                ]),
              ),
            ),
          ]),
        ),
      ),
    );
  }
}

const EdgeInsets _pad = EdgeInsets.all(TraceSpacing.gutter);

class _Overview extends ConsumerWidget {
  const _Overview(this.d, this.summary, this.key_);
  final WalletDetail d;
  final WalletSummary? summary;
  final AssetKey key_;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dna = ref.watch(walletDnaProvider(key_));
    return ListView(padding: _pad, children: [
      EnvelopeBar(d.env),
      TraceCard(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(shortAddress(d.wallet.address, head: 6, tail: 6), style: const TextStyle(fontWeight: FontWeight.w700)),
          const SizedBox(height: 4),
          Muted('First seen ${relativeTime(d.firstSeen)}. Last active ${relativeTime(d.lastSeen)}.'),
          Muted('Monitoring: ${d.monitoring.isEmpty ? 'unknown' : d.monitoring}'),
        ]),
      ),
      if (summary != null) ...[
        if (summary!.tags.isNotEmpty) ...[
          const SectionTitle('Strategy tags'),
          Wrap(spacing: 6, runSpacing: 6, children: [for (final t in summary!.tags) Pill(t)]),
          const SizedBox(height: 4),
          const Muted('Tags describe past behaviour that TRACE observed. They are not advice.', size: 12),
        ],
        const SizedBox(height: 8),
        StatGrid([
          StatTile(
            label: 'Past realized return',
            value: summary!.realizedReturnPct == null ? 'Not available' : signedPercentString(summary!.realizedReturnPct, decimals: 0),
            note: Copy.estimateLabel,
            valueColor: changeColor(parseDecimal(summary!.realizedReturnPct)),
          ),
          StatTile(label: 'Win rate', value: plainPercentString(summary!.winRatePct, decimals: 0), note: 'Past trades only'),
        ]),
      ],
      const SectionTitle('Trading DNA'),
      dna.when(
        loading: () => const Center(child: Padding(padding: EdgeInsets.all(16), child: CircularProgressIndicator())),
        error: (e, _) => ErrorState(error: e, onRetry: () => ref.invalidate(walletDnaProvider(key_))),
        data: (r) {
          if (r.insufficientHistory) {
            return TraceCard(
              semanticsLabel: 'Trading DNA: insufficient history. Only ${r.sampleSize} trades.',
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                const Pill('Insufficient history', color: TraceColors.warning),
                const SizedBox(height: 8),
                Text('TRACE has seen only ${r.sampleSize} completed trades, which is too few to describe a trading style.'),
                const Muted('We would rather show nothing than guess.', size: 12),
              ]),
            );
          }
          return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            EnvelopeBar(r.env),
            for (final t in r.traits)
              TraceCard(
                semanticsLabel: '${t.label}. Evidence: ${t.evidence}',
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text(t.label, style: const TextStyle(fontWeight: FontWeight.w700)),
                  const SizedBox(height: 4),
                  Muted('Evidence: ${t.evidence}'),
                ]),
              ),
            Muted('Based on ${r.sampleSize} trades. Rules version ${r.rulesVersion}. Labels come from explainable rules and describe only on-chain behaviour.', size: 12),
          ]);
        },
      ),
      const Disclaimer(),
    ]);
  }
}

class _Trades extends ConsumerWidget {
  const _Trades(this.k);
  final AssetKey k;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(walletTradesProvider(k));
    return AsyncBody<TradesResponse>(
      value: v,
      onRetry: () => ref.invalidate(walletTradesProvider(k)),
      builder: (r) => ListView(padding: _pad, children: [
        EnvelopeBar(r.env),
        if (r.items.isEmpty) const EmptyState(title: 'No trades seen', message: 'TRACE has not seen supported swaps from this wallet yet.'),
        for (final t in r.items)
          TraceCard(
            onTap: () => context.push(t.token.path),
            semanticsLabel: '${t.isBuy ? 'Bought' : 'Sold'} ${t.token.displayName}, ${compactUsdString(t.amountUsd)}, ${relativeTime(t.eventTime)}, ${t.finality == Finality.final_ ? 'final' : 'not yet final'}',
            child: Row(children: [
              TokenAvatar(t.token.displayName),
              const SizedBox(width: 12),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text('${t.isBuy ? 'Bought' : 'Sold'} ${t.token.displayName}',
                      style: TextStyle(fontWeight: FontWeight.w700, color: t.isBuy ? TraceColors.text : TraceColors.negative)),
                  Muted(relativeTime(t.eventTime), size: 12),
                ]),
              ),
              Column(crossAxisAlignment: CrossAxisAlignment.end, children: [
                Text(compactUsdString(t.amountUsd), style: const TextStyle(fontWeight: FontWeight.w700)),
                Pill(t.finality == Finality.final_ ? 'Final' : 'Not final yet',
                    color: t.finality == Finality.final_ ? TraceColors.positive : TraceColors.warning),
              ]),
            ]),
          ),
      ]),
    );
  }
}

class _Performance extends ConsumerWidget {
  const _Performance(this.k);
  final AssetKey k;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(walletMetricsProvider(k));
    return AsyncBody<WalletMetricsResponse>(
      value: v,
      onRetry: () => ref.invalidate(walletMetricsProvider(k)),
      builder: (r) {
        final m = r.metrics;
        return ListView(padding: _pad, children: [
          EnvelopeBar(r.env),
          Row(children: [
            Pill(confidenceText(m.pnlConfidence), color: confidenceColor(m.pnlConfidence)),
            const SizedBox(width: 8),
            const Expanded(child: Muted('PnL is computed by the TRACE server, not on your phone.', size: 12)),
          ]),
          const SizedBox(height: 12),
          StatGrid([
            StatTile(label: 'Realized PnL', value: m.realizedPnlUsd == null ? 'Unresolved' : compactUsdString(m.realizedPnlUsd), note: Copy.estimateLabel),
            StatTile(label: 'Unrealized PnL', value: m.unrealizedPnlUsd == null ? 'Unresolved' : compactUsdString(m.unrealizedPnlUsd), note: Copy.estimateLabel),
            StatTile(label: 'Total trades', value: compactCount(m.totalTrades)),
            StatTile(label: 'Price coverage', value: coverageText(m.coverageRatio), note: 'Share of trades TRACE could price'),
            StatTile(label: 'Unresolved sells', value: compactCount(m.unresolvedDisposals), note: 'Missing cost basis'),
          ]),
          if (m.unresolvedDisposals > 0)
            const Muted('Sells with a missing cost basis are left out instead of guessed, so real results may differ.', size: 12),
          if (r.wqs != null) _ScoreCard('Wallet quality score', r.wqs!),
          if (r.wcs != null) _ScoreCard('Wallet confidence score', r.wcs!),
          const Disclaimer(),
        ]);
      },
    );
  }
}

class _ScoreCard extends StatelessWidget {
  const _ScoreCard(this.title, this.s);
  final String title;
  final Score s;

  @override
  Widget build(BuildContext context) => TraceCard(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          const Pill(Copy.modelLabel, color: TraceColors.info),
          const SizedBox(height: 8),
          Text(title, style: const TextStyle(fontWeight: FontWeight.w700)),
          Text(s.value == null ? 'Not scored' : s.value!.toStringAsFixed(0),
              style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
          if (s.explanation.isNotEmpty) Muted(s.explanation),
          Muted('Version ${s.version}. Cohort: ${s.cohort}. Window: ${s.window}.', size: 11),
        ]),
      );
}
