import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../core/copy.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

class TokenScreen extends ConsumerWidget {
  const TokenScreen({super.key, required this.chain, required this.address});
  final Chain chain;
  final String address;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final key = (chain, address);
    final v = ref.watch(tokenProvider(key));
    final title = v.valueOrNull?.token.displayName ?? shortAddress(address);
    return DefaultTabController(
      length: 4,
      child: TraceScaffold(
        title: title,
        subtitle: chain.label,
        bottom: const TabBar(
          isScrollable: true,
          tabs: [
            Tab(text: 'Overview'),
            Tab(text: 'Analysis'),
            Tab(text: 'Buyers'),
            Tab(text: 'News'),
          ],
        ),
        body: AsyncBody<TokenDetail>(
          value: v,
          onRetry: () => ref.invalidate(tokenProvider(key)),
          builder: (t) => TabBarView(children: [
            _Overview(t),
            _Analysis(t),
            _Buyers(t),
            const _News(),
          ]),
        ),
      ),
    );
  }
}

EdgeInsets get _pad => const EdgeInsets.all(TraceSpacing.gutter);

class _Header extends StatelessWidget {
  const _Header(this.t);
  final TokenDetail t;

  @override
  Widget build(BuildContext context) {
    final change = parseDecimal(t.changePct24h);
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      EnvelopeBar(t.env),
      Semantics(
        container: true,
        label: '${t.token.displayName} price ${compactUsdString(t.priceUsd)}, '
            '${signedPercent(change)} over 24 hours',
        child: ExcludeSemantics(
          child: Row(children: [
            TokenAvatar(t.token.displayName, size: 48),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(t.token.name ?? t.token.displayName,
                        style: const TextStyle(color: TraceColors.textMuted)),
                    Wrap(
                        crossAxisAlignment: WrapCrossAlignment.center,
                        spacing: 8,
                        children: [
                          Text(compactUsdString(t.priceUsd),
                              style: const TextStyle(
                                  fontSize: 28, fontWeight: FontWeight.w800)),
                          Text(signedPercent(change),
                              style: TextStyle(
                                  fontSize: 18,
                                  fontWeight: FontWeight.w700,
                                  color: changeColor(change))),
                          const Muted('24h', size: 12),
                        ]),
                  ]),
            ),
          ]),
        ),
      ),
      const SizedBox(height: 12),
    ]);
  }
}

class _Overview extends StatelessWidget {
  const _Overview(this.t);
  final TokenDetail t;

  @override
  Widget build(BuildContext context) {
    return ListView(padding: _pad, children: [
      _Header(t),
      StatGrid([
        StatTile(label: 'Market cap', value: compactUsdString(t.marketCapUsd)),
        StatTile(
            label: 'Volume (24h)', value: compactUsdString(t.volume24hUsd)),
        StatTile(
            label: 'Liquidity',
            value: compactUsdString(t.liquidityUsd),
            note: 'Money in the trading pool'),
        StatTile(label: 'Holders', value: compactCount(t.holders)),
      ]),
      const Muted('Price chart: not available in this version.', size: 12),
      const SectionTitle("Why it's moving"),
      if (t.whyMoving.isEmpty)
        const TraceCard(
            child: Muted(
                'No reason found. TRACE only lists reasons it can back with evidence.')),
      for (final w in t.whyMoving)
        TraceCard(
          semanticsLabel:
              '${w.kind == EvidenceKind.fact ? Copy.factLabel : Copy.hypothesisLabel}. ${w.text}',
          child:
              Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            KindLabel(w.kind),
            const SizedBox(height: 8),
            Text(w.text),
            if (w.evidenceIds.isNotEmpty)
              Muted('Evidence items: ${w.evidenceIds.length}', size: 12),
          ]),
        ),
      SectionTitle('Risk', trailing: RiskPill(t.risk.overall)),
      TraceCard(
        onTap: () =>
            context.push('/risk/${t.token.chain.wire}/${t.token.address}'),
        semanticsLabel: 'Open Risk Radar',
        child: const Row(children: [
          Icon(Icons.shield_outlined, color: TraceColors.emerald),
          SizedBox(width: 12),
          Expanded(child: Text('See the full Risk Radar')),
          Icon(Icons.chevron_right),
        ]),
      ),
      Row(children: [
        Expanded(
          child: OutlinedButton(
            onPressed: () => context
                .push('/compare?a=${t.token.chain.wire}:${t.token.address}'),
            child: const Text('Compare'),
          ),
        ),
        const SizedBox(width: 12),
        const Expanded(
            child: OutlinedButton(
                onPressed: null,
                child:
                    Text('Buy (Coming later)', textAlign: TextAlign.center))),
      ]),
      const Disclaimer(),
    ]);
  }
}

class _Analysis extends StatelessWidget {
  const _Analysis(this.t);
  final TokenDetail t;

  @override
  Widget build(BuildContext context) {
    final p = '${t.token.chain.wire}/${t.token.address}';
    Widget link(IconData i, String title, String sub, String path) => TraceCard(
          onTap: () => context.push(path),
          semanticsLabel: '$title. $sub',
          child: Row(children: [
            Icon(i, color: TraceColors.emerald),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title,
                        style: const TextStyle(fontWeight: FontWeight.w700)),
                    Muted(sub, size: 12),
                  ]),
            ),
            const Icon(Icons.chevron_right),
          ]),
        );
    return ListView(padding: _pad, children: [
      EnvelopeBar(t.env),
      link(Icons.timeline, 'Opportunity Timeline', 'What happened, in order',
          '/timeline/$p'),
      link(Icons.shield_outlined, 'Risk Radar',
          'Checks for scams and thin liquidity', '/risk/$p'),
      link(Icons.compare_arrows, 'Compare coins', 'Put two coins side by side',
          '/compare?a=${t.token.chain.wire}:${t.token.address}'),
      const Disclaimer(),
    ]);
  }
}

class _Buyers extends StatelessWidget {
  const _Buyers(this.t);
  final TokenDetail t;

  @override
  Widget build(BuildContext context) {
    return ListView(padding: _pad, children: [
      EnvelopeBar(t.env),
      const Muted(
          'Tracked wallets that bought this token recently (observed).'),
      const SizedBox(height: 8),
      if (t.buyers.isEmpty)
        const EmptyState(
            title: 'No tracked buyers',
            message:
                'No tracked wallet bought this token in the recent window.'),
      for (final w in t.buyers)
        TraceCard(
          onTap: () => context.push(w.path),
          semanticsLabel:
              'Wallet ${w.displayName} on ${w.chain.label}. Open profile.',
          child: Row(children: [
            const Icon(Icons.account_circle_outlined,
                color: TraceColors.emerald),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(w.displayName,
                        style: const TextStyle(fontWeight: FontWeight.w700)),
                    Muted(shortAddress(w.address), size: 12),
                  ]),
            ),
            const Icon(Icons.chevron_right),
          ]),
        ),
    ]);
  }
}

class _News extends StatelessWidget {
  const _News();
  @override
  Widget build(BuildContext context) =>
      ListView(padding: _pad, children: const [
        ComingLaterPanel(
          title: 'News and social',
          explanation:
              'News and social feeds are not part of this version. TRACE only shows what it can verify on-chain.',
          icon: Icons.newspaper,
        ),
      ]);
}
