import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

class BriefingScreen extends ConsumerWidget {
  const BriefingScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(briefingProvider);
    return TraceScaffold(
      title: 'Daily Briefing',
      subtitle: 'Your personalized market summary',
      body: AsyncBody<BriefingResponse>(
        value: v,
        onRetry: () => ref.invalidate(briefingProvider),
        builder: (r) => ListView(padding: const EdgeInsets.all(TraceSpacing.gutter), children: [
          EnvelopeBar(r.env),
          if (r.sections.isEmpty)
            const EmptyState(title: 'No briefing yet', message: 'Nothing to summarize right now. Check back later.'),
          for (final s in r.sections) ...[
            SectionTitle(s.title),
            for (final i in s.items)
              TraceCard(
                onTap: (i.link != null && i.link!.startsWith('/')) ? () => context.go(i.link!) : null,
                semanticsLabel: i.text,
                child: Row(children: [
                  Expanded(child: Text(i.text)),
                  if (i.link != null && i.link!.startsWith('/')) const Icon(Icons.chevron_right),
                ]),
              ),
          ],
          const Disclaimer(),
        ]),
      ),
    );
  }
}
