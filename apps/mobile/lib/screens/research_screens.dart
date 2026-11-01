import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../core/copy.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

// ---------------------------------------------------------------------------
// Opportunity Timeline
// ---------------------------------------------------------------------------

class TimelineScreen extends ConsumerWidget {
  const TimelineScreen({super.key, required this.chain, required this.address});
  final Chain chain;
  final String address;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final key = (chain, address);
    final v = ref.watch(timelineProvider(key));
    final tok = ref.watch(tokenProvider(key)).valueOrNull;
    return TraceScaffold(
      title: 'Opportunity Timeline',
      subtitle:
          '${tok?.token.displayName ?? shortAddress(address)}: what happened, in order',
      body: AsyncBody<TimelineResponse>(
        value: v,
        onRetry: () => ref.invalidate(timelineProvider(key)),
        builder: (r) => ListView(
            padding: const EdgeInsets.all(TraceSpacing.gutter),
            children: [
              EnvelopeBar(r.env),
              const Muted(
                  'Facts are things GiFTED! observed. Hypotheses are possible causes and are not confirmed.',
                  size: 12),
              const SizedBox(height: 12),
              if (r.items.isEmpty)
                const EmptyState(
                    title: 'No events yet',
                    message:
                        'GiFTED! has not recorded events for this token in this window.'),
              for (var i = 0; i < r.items.length; i++)
                _Step(r.items[i], last: i == r.items.length - 1),
              const Disclaimer(),
            ]),
      ),
    );
  }
}

class _Step extends StatelessWidget {
  const _Step(this.item, {required this.last});
  final TimelineItem item;
  final bool last;

  @override
  Widget build(BuildContext context) {
    final fact = item.kind == EvidenceKind.fact;
    final color = fact ? TraceColors.positive : TraceColors.info;
    return Semantics(
      container: true,
      label:
          '${clockUtc(item.time)}. ${fact ? Copy.factLabel : Copy.hypothesisLabel}. ${item.title}. ${item.detail}'
          '${item.possibleImpact == null ? '' : ' Possible impact: ${item.possibleImpact}.'}',
      child: ExcludeSemantics(
        child: IntrinsicHeight(
          child: Row(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
            SizedBox(
              width: 28,
              child: Column(children: [
                Container(
                  width: 14,
                  height: 14,
                  margin: const EdgeInsets.only(top: 4),
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: fact ? color : TraceColors.bg,
                    border: Border.all(color: color, width: 2),
                  ),
                ),
                if (!last)
                  Expanded(
                      child: Container(width: 2, color: TraceColors.border)),
              ]),
            ),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.only(bottom: 16),
                child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Muted(
                          '${clockUtc(item.time)}  (${relativeTime(item.time)})',
                          size: 12),
                      const SizedBox(height: 4),
                      KindLabel(item.kind),
                      const SizedBox(height: 4),
                      Text(item.title,
                          style: const TextStyle(fontWeight: FontWeight.w700)),
                      if (item.detail.isNotEmpty) Muted(item.detail),
                      if (item.possibleImpact != null)
                        Muted('Possible impact: ${item.possibleImpact}',
                            size: 12),
                      if (item.evidenceIds.isNotEmpty)
                        Muted('Evidence items: ${item.evidenceIds.length}',
                            size: 12)
                      else if (fact)
                        const Muted('No evidence id attached', size: 12),
                    ]),
              ),
            ),
          ]),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Risk Radar
// ---------------------------------------------------------------------------

class RiskRadarScreen extends ConsumerWidget {
  const RiskRadarScreen(
      {super.key, required this.chain, required this.address});
  final Chain chain;
  final String address;

  static String overallText(RiskState s) => switch (s) {
        RiskState.pass => 'No warnings found',
        RiskState.warn => 'Some warnings',
        RiskState.block => 'Serious warnings',
        RiskState.unknown => 'Not enough data',
      };

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final key = (chain, address);
    final v = ref.watch(tokenProvider(key));
    return TraceScaffold(
      title: 'Risk Radar',
      subtitle: 'Spot possible risks before you trade',
      body: AsyncBody<TokenDetail>(
        value: v,
        onRetry: () => ref.invalidate(tokenProvider(key)),
        builder: (t) => ListView(
            padding: const EdgeInsets.all(TraceSpacing.gutter),
            children: [
              EnvelopeBar(t.env),
              Center(
                child: Semantics(
                  label:
                      'Overall risk checks for ${t.token.displayName}: ${overallText(t.risk.overall)}',
                  child: ExcludeSemantics(
                    child: Column(children: [
                      CustomPaint(
                          size: const Size(180, 100),
                          painter: _GaugePainter(t.risk.overall)),
                      Text(overallText(t.risk.overall),
                          style: TextStyle(
                              fontSize: 20,
                              fontWeight: FontWeight.w800,
                              color: riskColor(t.risk.overall))),
                      Muted('Checked ${relativeTime(t.risk.fetchedAt)}',
                          size: 12),
                    ]),
                  ),
                ),
              ),
              const SizedBox(height: 12),
              if (t.risk.checks.isEmpty)
                const EmptyState(
                    title: 'No checks available',
                    message: 'Risk checks could not be run for this token.'),
              for (final c in t.risk.checks)
                TraceCard(
                  semanticsLabel: '${c.label}: ${riskWord(c.state)}. ${c.note}',
                  child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(children: [
                          Expanded(
                              child: Text(c.label,
                                  style: const TextStyle(
                                      fontWeight: FontWeight.w700))),
                          RiskPill(c.state),
                        ]),
                        if (c.note.isNotEmpty) Muted(c.note),
                      ]),
                ),
              const SectionTitle('How to read this'),
              const TraceCard(
                child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Pill(Copy.riskLabel, color: TraceColors.warning),
                      SizedBox(height: 8),
                      Text(
                          'A passing check is not a safety guarantee. Checks can miss new scams, and low liquidity can make prices swing even when every check passes.'),
                    ]),
              ),
              const Disclaimer(),
            ]),
      ),
    );
  }
}

class _GaugePainter extends CustomPainter {
  _GaugePainter(this.state);
  final RiskState state;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Rect.fromLTWH(10, 10, size.width - 20, (size.height - 10) * 2);
    final track = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 14
      ..strokeCap = StrokeCap.round
      ..color = TraceColors.border;
    canvas.drawArc(rect, math.pi, math.pi, false, track);
    final fraction = switch (state) {
      RiskState.pass => 0.25,
      RiskState.warn => 0.6,
      RiskState.block => 1.0,
      RiskState.unknown => 0.0,
    };
    if (fraction > 0) {
      canvas.drawArc(rect, math.pi, math.pi * fraction, false,
          track..color = riskColor(state));
    }
  }

  @override
  bool shouldRepaint(covariant _GaugePainter old) => old.state != state;
}

// ---------------------------------------------------------------------------
// Compare Coins
// ---------------------------------------------------------------------------

class CompareScreen extends ConsumerStatefulWidget {
  const CompareScreen({super.key, this.a, this.b});

  /// Optional "chain:address" values from the route.
  final String? a;
  final String? b;

  @override
  ConsumerState<CompareScreen> createState() => _CompareScreenState();
}

class _CompareScreenState extends ConsumerState<CompareScreen> {
  TokenRef? _a;
  TokenRef? _b;

  TokenRef? _parse(String? s) {
    if (s == null) return null;
    final i = s.indexOf(':');
    if (i < 1) return null;
    final c = Chain.parse(s.substring(0, i));
    if (c == Chain.unknown) return null;
    return TokenRef(chain: c, address: s.substring(i + 1));
  }

  @override
  void initState() {
    super.initState();
    _a = _parse(widget.a);
    _b = _parse(widget.b);
  }

  @override
  Widget build(BuildContext context) {
    final movers = ref.watch(moversProvider(('trending', '24h', '')));
    return TraceScaffold(
      title: 'Compare Coins',
      subtitle: 'See which coin looks stronger, in plain English',
      body: movers.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => ErrorState(
            error: e,
            onRetry: () =>
                ref.invalidate(moversProvider(('trending', '24h', '')))),
        data: (m) {
          final tokens = m.items.map((e) => e.token).toList();
          if (tokens.length < 2) {
            return const EmptyState(
                title: 'Not enough coins',
                message: 'Compare needs at least two coins from Top Movers.');
          }
          _a ??= tokens[0];
          _b ??= tokens.firstWhere((t) => t.address != _a!.address,
              orElse: () => tokens[1]);
          final options = <TokenRef>[...tokens];
          for (final x in [_a!, _b!]) {
            if (!options.any(
              (t) => t.address == x.address && t.chain == x.chain,
            )) {
              options.insert(0, x);
            }
          }
          return ListView(
              padding: const EdgeInsets.all(TraceSpacing.gutter),
              children: [
                _Picker(
                    'First coin', options, _a!, (t) => setState(() => _a = t)),
                _Picker(
                    'Second coin', options, _b!, (t) => setState(() => _b = t)),
                const SizedBox(height: 8),
                _CompareResult(a: _a!, b: _b!),
              ]);
        },
      ),
    );
  }
}

class _Picker extends StatelessWidget {
  const _Picker(this.label, this.options, this.value, this.onChanged);
  final String label;
  final List<TokenRef> options;
  final TokenRef value;
  final ValueChanged<TokenRef> onChanged;

  @override
  Widget build(BuildContext context) {
    final selected = options.firstWhere(
      (t) => t.address == value.address && t.chain == value.chain,
      orElse: () => options.first,
    );
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: DropdownButtonFormField<TokenRef>(
        initialValue: selected,
        isExpanded: true,
        decoration: InputDecoration(labelText: label),
        items: [
          for (final t in options)
            DropdownMenuItem(value: t, child: Text(t.displayName)),
        ],
        onChanged: (t) {
          if (t != null) onChanged(t);
        },
      ),
    );
  }
}

class _CompareResult extends ConsumerWidget {
  const _CompareResult({required this.a, required this.b});
  final TokenRef a;
  final TokenRef b;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final key = (a.chain, a.address, b.chain, b.address);
    final v = ref.watch(compareProvider(key));
    return v.when(
      loading: () => const Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: CircularProgressIndicator())),
      error: (e, _) => SizedBox(
          height: 280,
          child: ErrorState(
              error: e, onRetry: () => ref.invalidate(compareProvider(key)))),
      data: (r) {
        final items = r.items;
        Widget row(String label, String Function(TokenSummary) f,
            {Color? Function(TokenSummary)? color}) {
          return Padding(
            padding: const EdgeInsets.symmetric(vertical: 6),
            child: Semantics(
              container: true,
              label:
                  '$label: ${items.map((t) => '${t.token.displayName} ${f(t)}').join(', ')}',
              child: ExcludeSemantics(
                child: Row(children: [
                  Expanded(flex: 3, child: Muted(label, size: 12)),
                  for (final t in items)
                    Expanded(
                      flex: 3,
                      child: Text(f(t),
                          style: TextStyle(
                              fontWeight: FontWeight.w700,
                              color: color?.call(t))),
                    ),
                ]),
              ),
            ),
          );
        }

        return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          EnvelopeBar(r.env),
          TraceCard(
            child: Column(children: [
              Row(children: [
                const Expanded(flex: 3, child: SizedBox.shrink()),
                for (final t in items)
                  Expanded(
                      flex: 3,
                      child: Text(t.token.displayName,
                          style: const TextStyle(
                              fontWeight: FontWeight.w800, fontSize: 16))),
              ]),
              const Divider(),
              row('Price change (24h)',
                  (t) => signedPercentString(t.changePct24h),
                  color: (t) => changeColor(parseDecimal(t.changePct24h))),
              row('Price', (t) => compactUsdString(t.priceUsd)),
              row('Market cap', (t) => compactUsdString(t.marketCapUsd)),
              row('Volume (24h)', (t) => compactUsdString(t.volume24hUsd)),
              row('Liquidity', (t) => compactUsdString(t.liquidityUsd)),
              row('Risk checks', (t) => riskWord(t.risk),
                  color: (t) => riskColor(t.risk)),
            ]),
          ),
          if (r.verdictNote.isNotEmpty)
            TraceCard(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Pill(Copy.modelLabel, color: TraceColors.info),
                    const SizedBox(height: 8),
                    Text(r.verdictNote),
                  ]),
            ),
          const Disclaimer(),
        ]);
      },
    );
  }
}

// ---------------------------------------------------------------------------
// Token picker used by More for screens that need a token
// ---------------------------------------------------------------------------

Future<void> pickTokenAndGo(
    BuildContext context, WidgetRef ref, String routePrefix) async {
  final res =
      await ref.read(moversProvider(('trending', '24h', '')).future).catchError(
            (_) => const MoversResponse(
                env: Envelope(status: DataStatus.unavailable), items: []),
          );
  if (!context.mounted) return;
  final tokens = res.items.map((e) => e.token).toList();
  if (tokens.isEmpty) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
          content: Text('No tokens available right now. Try again later.')),
    );
    return;
  }
  final picked = await showModalBottomSheet<TokenRef>(
    context: context,
    backgroundColor: TraceColors.surface,
    builder: (ctx) => SafeArea(
      child: ListView(shrinkWrap: true, children: [
        const Padding(
          padding: EdgeInsets.all(16),
          child: Text('Choose a coin',
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
        ),
        for (final t in tokens)
          ListTile(
            minVerticalPadding: 12,
            title: Text(t.displayName),
            subtitle: Text(t.chain.label),
            onTap: () => Navigator.of(ctx).pop(t),
          ),
      ]),
    ),
  );
  if (picked != null && context.mounted) {
    context.push('$routePrefix/${picked.chain.wire}/${picked.address}');
  }
}
