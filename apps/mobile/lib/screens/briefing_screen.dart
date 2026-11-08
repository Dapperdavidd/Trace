import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

/// The daily briefing: a written report built from real on-chain activity. Text only.
class BriefingScreen extends ConsumerWidget {
  const BriefingScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(briefingProvider);
    return TraceScaffold(
      title: 'Daily briefing',
      subtitle: 'Written from the last 24 hours on chain',
      body: AsyncBody<BriefingResponse>(
        value: v,
        onRetry: () => ref.invalidate(briefingProvider),
        builder: (r) {
          final short = r.sections.where((s) => s.title == 'The short version').toList();
          final rest = r.sections.where((s) => s.title != 'The short version');
          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
            children: [
              if (r.sections.isEmpty)
                EmptyState(
                    title: 'Not enough activity yet',
                    message: r.env.coverageNote ??
                        'The report fills in as trace reads more trades.'),
              for (final s in short)
                Container(
                  padding: const EdgeInsets.all(18),
                  margin: const EdgeInsets.only(bottom: 16),
                  decoration: BoxDecoration(
                      color: TraceColors.emeraldDeep,
                      borderRadius: BorderRadius.circular(TraceSpacing.radius)),
                  child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                    const Text('The short version',
                        style: TextStyle(color: TraceColors.textMuted, fontSize: 13)),
                    const SizedBox(height: 8),
                    Text(s.items.map((i) => i.text).join(' '),
                        style: const TextStyle(fontSize: 16, height: 1.45)),
                  ]),
                ),
              for (final s in rest) ...[
                SectionTitle(s.title),
                for (final i in s.items)
                  TraceCard(
                    onTap: i.route != null && (i.token != null || i.wallet != null)
                        ? () => context.push(i.route!)
                        : null,
                    semanticsLabel: i.text,
                    child: Row(children: [
                      Expanded(child: Text(i.text)),
                      if (i.token != null || i.wallet != null)
                        const Icon(Icons.chevron_right),
                    ]),
                  ),
              ],
              const Disclaimer(
                  text: 'Facts from finalized on-chain trades, prices and safety checks. Not financial advice.'),
            ],
          );
        },
      ),
    );
  }
}
