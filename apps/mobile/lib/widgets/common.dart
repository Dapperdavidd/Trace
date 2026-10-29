import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/api_error.dart';
import '../api/models.dart';
import '../core/copy.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import '../state/providers.dart';

/// Scaffold used by every screen. Always shows the global DEMO DATA banner
/// when the app is serving demo fixtures.
class TraceScaffold extends ConsumerWidget {
  const TraceScaffold({
    super.key,
    required this.title,
    required this.body,
    this.subtitle,
    this.actions,
    this.bottom,
    this.floatingActionButton,
  });

  final String title;
  final String? subtitle;
  final Widget body;
  final List<Widget>? actions;
  final PreferredSizeWidget? bottom;
  final Widget? floatingActionButton;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final demo = ref.watch(demoActiveProvider);
    return Scaffold(
      appBar: AppBar(
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Semantics(header: true, child: Text(title, style: const TextStyle(fontWeight: FontWeight.w700))),
            if (subtitle != null)
              Text(subtitle!, style: const TextStyle(fontSize: 12, color: TraceColors.textMuted)),
          ],
        ),
        actions: actions,
        bottom: bottom,
      ),
      floatingActionButton: floatingActionButton,
      body: Column(
        children: [
          ValueListenableBuilder<bool>(
            valueListenable: demo,
            builder: (context, active, _) => active ? const DemoBanner() : const SizedBox.shrink(),
          ),
          Expanded(child: body),
        ],
      ),
    );
  }
}

class DemoBanner extends StatelessWidget {
  const DemoBanner({super.key});

  @override
  Widget build(BuildContext context) {
    return Semantics(
      container: true,
      label: 'Demo data. ${Copy.demoExplain}',
      child: Container(
        width: double.infinity,
        color: TraceColors.warning,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: const ExcludeSemantics(
          child: Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 8,
            children: [
              Text(Copy.demoBanner,
                  style: TextStyle(
                      color: Color(0xFF241A00), fontWeight: FontWeight.w900, letterSpacing: 1)),
              Text('Fictional example data. Not live.',
                  style: TextStyle(color: Color(0xFF241A00), fontSize: 13)),
            ],
          ),
        ),
      ),
    );
  }
}

/// Per-response status bar: shows the data status and as-of time.
class EnvelopeBar extends StatelessWidget {
  const EnvelopeBar(this.env, {super.key});
  final Envelope env;

  @override
  Widget build(BuildContext context) {
    final (label, color, explain) = switch (env.status) {
      DataStatus.live => ('LIVE', TraceColors.positive, 'Live data'),
      DataStatus.stale => ('STALE', TraceColors.warning, 'This data is out of date. Do not treat it as current.'),
      DataStatus.unavailable => ('UNAVAILABLE', TraceColors.negative, 'The data source is not available right now.'),
      DataStatus.partial => ('PARTIAL', TraceColors.warning, 'Some data is missing, so this view is incomplete.'),
      DataStatus.demo => (Copy.demoBanner, TraceColors.warning, Copy.demoExplain),
    };
    return Semantics(
      container: true,
      label: 'Data status: $label. $explain. Updated ${relativeTime(env.asOf)}.',
      child: ExcludeSemantics(
        child: Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Wrap(
                spacing: 8,
                runSpacing: 4,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  Pill(label, color: color, filled: env.status == DataStatus.demo),
                  Text('Updated ${relativeTime(env.asOf)}',
                      style: const TextStyle(color: TraceColors.textMuted, fontSize: 12)),
                ],
              ),
              if (env.status != DataStatus.live)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: Text(explain, style: const TextStyle(color: TraceColors.textMuted, fontSize: 12)),
                ),
              if (env.coverageNote != null && env.coverageNote!.isNotEmpty && !env.isDemo)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: Text('Coverage: ${env.coverageNote}',
                      style: const TextStyle(color: TraceColors.textMuted, fontSize: 12)),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class Pill extends StatelessWidget {
  const Pill(this.label, {super.key, this.color = TraceColors.emerald, this.filled = false});
  final String label;
  final Color color;
  final bool filled;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: filled ? color : tint(color, 0.16),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: tint(color, 0.6)),
      ),
      child: Text(label,
          style: TextStyle(
            color: filled ? TraceColors.onEmerald : color,
            fontWeight: FontWeight.w700,
            fontSize: 12,
          )),
    );
  }
}

Color riskColor(RiskState r) => switch (r) {
      RiskState.pass => TraceColors.positive,
      RiskState.warn => TraceColors.warning,
      RiskState.block => TraceColors.negative,
      RiskState.unknown => TraceColors.textMuted,
    };

String riskWord(RiskState r) => switch (r) {
      RiskState.pass => 'No warning',
      RiskState.warn => 'Caution',
      RiskState.block => 'High risk',
      RiskState.unknown => 'Unknown',
    };

class RiskPill extends StatelessWidget {
  const RiskPill(this.state, {super.key});
  final RiskState state;

  @override
  Widget build(BuildContext context) => Semantics(
        label: 'Risk check: ${riskWord(state)}',
        child: ExcludeSemantics(child: Pill(riskWord(state), color: riskColor(state))),
      );
}

/// Separates observed fact from hypothesis.
class KindLabel extends StatelessWidget {
  const KindLabel(this.kind, {super.key});
  final EvidenceKind kind;

  @override
  Widget build(BuildContext context) {
    final fact = kind == EvidenceKind.fact;
    return Pill(fact ? Copy.factLabel : Copy.hypothesisLabel,
        color: fact ? TraceColors.positive : TraceColors.info);
  }
}

class TraceCard extends StatelessWidget {
  const TraceCard({super.key, required this.child, this.onTap, this.semanticsLabel, this.padding});
  final Widget child;
  final VoidCallback? onTap;
  final String? semanticsLabel;
  final EdgeInsetsGeometry? padding;

  @override
  Widget build(BuildContext context) {
    final content = Padding(padding: padding ?? const EdgeInsets.all(16), child: child);
    final box = Material(
      color: TraceColors.surface,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(TraceSpacing.radius),
        side: const BorderSide(color: TraceColors.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: onTap == null
          ? content
          : InkWell(
              onTap: onTap,
              child: ConstrainedBox(
                constraints: const BoxConstraints(minHeight: TraceSpacing.minTouch),
                child: content,
              ),
            ),
    );
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: semanticsLabel == null
          ? box
          : Semantics(label: semanticsLabel, button: onTap != null, container: true, onTap: onTap, child: ExcludeSemantics(child: box)),
    );
  }
}

class SectionTitle extends StatelessWidget {
  const SectionTitle(this.text, {super.key, this.trailing});
  final String text;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(0, 8, 0, 8),
        child: Row(
          children: [
            Expanded(
              child: Semantics(
                header: true,
                child: Text(text, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
              ),
            ),
            if (trailing != null) trailing!,
          ],
        ),
      );
}

class Muted extends StatelessWidget {
  const Muted(this.text, {super.key, this.size = 13});
  final String text;
  final double size;
  @override
  Widget build(BuildContext context) =>
      Text(text, style: TextStyle(color: TraceColors.textMuted, fontSize: size));
}

class Disclaimer extends StatelessWidget {
  const Disclaimer({super.key, this.text = Copy.notAdvice});
  final String text;
  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 12),
        child: Muted(text, size: 12),
      );
}

class StatTile extends StatelessWidget {
  const StatTile({super.key, required this.label, required this.value, this.note, this.valueColor});
  final String label;
  final String value;
  final String? note;
  final Color? valueColor;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      container: true,
      label: '$label: $value${note == null ? '' : '. $note'}',
      child: ExcludeSemantics(
        child: Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: TraceColors.surface,
            borderRadius: BorderRadius.circular(14),
            border: Border.all(color: TraceColors.border),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Muted(label, size: 12),
              const SizedBox(height: 4),
              Text(value,
                  style: TextStyle(fontSize: 20, fontWeight: FontWeight.w800, color: valueColor ?? TraceColors.text)),
              if (note != null) Muted(note!, size: 11),
            ],
          ),
        ),
      ),
    );
  }
}

/// Two-column grid that survives large text scale.
class StatGrid extends StatelessWidget {
  const StatGrid(this.tiles, {super.key});
  final List<Widget> tiles;

  @override
  Widget build(BuildContext context) {
    final wide = MediaQuery.textScalerOf(context).scale(1) <= 1.3;
    if (!wide) {
      return Column(children: [for (final t in tiles) Padding(padding: const EdgeInsets.only(bottom: 8), child: SizedBox(width: double.infinity, child: t))]);
    }
    final rows = <Widget>[];
    for (var i = 0; i < tiles.length; i += 2) {
      rows.add(Padding(
        padding: const EdgeInsets.only(bottom: 8),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Expanded(child: tiles[i]),
              const SizedBox(width: 8),
              Expanded(child: i + 1 < tiles.length ? tiles[i + 1] : const SizedBox.shrink()),
            ],
          ),
        ),
      ));
    }
    return Column(children: rows);
  }
}

class TokenAvatar extends StatelessWidget {
  const TokenAvatar(this.symbol, {super.key, this.size = 40});
  final String symbol;
  final double size;
  @override
  Widget build(BuildContext context) => ExcludeSemantics(
        child: Container(
          width: size,
          height: size,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: tint(TraceColors.emerald, 0.16),
            border: Border.all(color: tint(TraceColors.emerald, 0.5)),
          ),
          child: Text(symbol.isEmpty ? '?' : symbol.substring(0, 1).toUpperCase(),
              style: const TextStyle(fontWeight: FontWeight.w800, color: TraceColors.emerald)),
        ),
      );
}

Color changeColor(double? v) =>
    v == null ? TraceColors.textMuted : (v < 0 ? TraceColors.negative : TraceColors.positive);

/// "Coming later" badge and explanation for features outside the MVP.
class ComingLaterPanel extends StatelessWidget {
  const ComingLaterPanel({super.key, required this.title, required this.explanation, this.icon = Icons.lock_clock, this.children = const []});
  final String title;
  final String explanation;
  final IconData icon;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return TraceCard(
      semanticsLabel: '$title. ${Copy.comingLater}. $explanation',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(children: [
            Icon(icon, color: TraceColors.textMuted),
            const SizedBox(width: 12),
            Expanded(child: Text(title, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700))),
            const Pill(Copy.comingLater, color: TraceColors.info),
          ]),
          const SizedBox(height: 8),
          Muted(explanation),
          ...children,
        ],
      ),
    );
  }
}

class DisabledAction extends StatelessWidget {
  const DisabledAction({super.key, required this.label, this.icon});
  final String label;
  final IconData? icon;
  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.only(top: 8),
        child: Semantics(
          button: true,
          enabled: false,
          label: '$label. ${Copy.comingLater}',
          child: ExcludeSemantics(
            child: SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                onPressed: null,
                icon: Icon(icon ?? Icons.lock_outline),
                label: Text('$label (${Copy.comingLater})'),
              ),
            ),
          ),
        ),
      );
}

class ErrorState extends StatelessWidget {
  const ErrorState({super.key, required this.error, required this.onRetry});
  final Object error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final msg = error is ApiException ? (error as ApiException).message : 'Something went wrong.';
    final unreachable = error is ApiException && (error as ApiException).isUnreachable;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.cloud_off, size: 40, color: TraceColors.textMuted),
            const SizedBox(height: 12),
            const Text('Data unavailable', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
            const SizedBox(height: 8),
            Text(unreachable ? 'We could not reach the TRACE server.' : msg,
                textAlign: TextAlign.center, style: const TextStyle(color: TraceColors.textMuted)),
            const SizedBox(height: 16),
            FilledButton(onPressed: onRetry, child: const Text('Try again')),
          ],
        ),
      ),
    );
  }
}

class EmptyState extends StatelessWidget {
  const EmptyState({super.key, required this.title, required this.message, this.action});
  final String title;
  final String message;
  final Widget? action;
  @override
  Widget build(BuildContext context) => Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            Text(title, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700), textAlign: TextAlign.center),
            const SizedBox(height: 8),
            Text(message, textAlign: TextAlign.center, style: const TextStyle(color: TraceColors.textMuted)),
            if (action != null) ...[const SizedBox(height: 16), action!],
          ]),
        ),
      );
}

class AsyncBody<T> extends StatelessWidget {
  const AsyncBody({super.key, required this.value, required this.builder, required this.onRetry});
  final AsyncValue<T> value;
  final Widget Function(T data) builder;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => value.when(
        data: builder,
        loading: () => const Center(
          child: Semantics(label: 'Loading', child: CircularProgressIndicator()),
        ),
        error: (e, _) => ErrorState(error: e, onRetry: onRetry),
      );
}

/// Sparkline that respects the reduced motion setting.
class Sparkline extends StatelessWidget {
  const Sparkline(this.values, {super.key, this.width = 72, this.height = 28});
  final List<double> values;
  final double width;
  final double height;

  @override
  Widget build(BuildContext context) {
    if (values.length < 2) return SizedBox(width: width, height: height);
    final rising = values.last >= values.first;
    final color = rising ? TraceColors.positive : TraceColors.negative;
    final reduce = MediaQuery.disableAnimationsOf(context);
    return ExcludeSemantics(
      child: TweenAnimationBuilder<double>(
        tween: Tween(begin: reduce ? 1 : 0, end: 1),
        duration: reduce ? Duration.zero : const Duration(milliseconds: 600),
        builder: (context, t, _) => CustomPaint(
          size: Size(width, height),
          painter: _SparkPainter(values, color, t),
        ),
      ),
    );
  }
}

class _SparkPainter extends CustomPainter {
  _SparkPainter(this.values, this.color, this.progress);
  final List<double> values;
  final Color color;
  final double progress;

  @override
  void paint(Canvas canvas, Size size) {
    var lo = values.first, hi = values.first;
    for (final v in values) {
      if (v < lo) lo = v;
      if (v > hi) hi = v;
    }
    final span = (hi - lo) == 0 ? 1.0 : hi - lo;
    final path = Path();
    final count = (values.length * progress).ceil().clamp(2, values.length);
    for (var i = 0; i < count; i++) {
      final x = size.width * i / (values.length - 1);
      final y = size.height - ((values[i] - lo) / span) * (size.height - 4) - 2;
      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2
        ..strokeCap = StrokeCap.round,
    );
  }

  @override
  bool shouldRepaint(covariant _SparkPainter old) =>
      old.values != values || old.color != color || old.progress != progress;
}
