import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/models.dart';
import '../core/config.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

String alertTypeText(AlertType t) => switch (t) {
      AlertType.walletBuy => 'Wallet bought',
      AlertType.walletSell => 'Wallet sold',
      AlertType.riskEvent => 'Risk change',
      AlertType.signal => 'Signal',
      AlertType.unknown => 'Alert',
    };

class AlertsScreen extends ConsumerWidget {
  const AlertsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(alertsProvider);
    return TraceScaffold(
      title: 'Alerts',
      subtitle: 'Checked every ${AppConfig.alertPollSeconds} seconds while this screen is open',
      body: AsyncBody<AlertsPage>(
        value: v,
        onRetry: () => ref.invalidate(alertsProvider),
        builder: (page) => RefreshIndicator(
          onRefresh: () async => ref.invalidate(alertsProvider),
          child: ListView(padding: const EdgeInsets.all(TraceSpacing.gutter), children: [
            const ComingLaterPanel(
              title: 'Push notifications',
              explanation: 'Alerts appear here only. You will not get phone notifications yet.',
              icon: Icons.notifications_off_outlined,
            ),
            if (page.items.isEmpty)
              const EmptyState(title: 'No alerts yet', message: 'When a watched wallet trades or a risk check changes, it shows up here.'),
            for (final a in page.items)
              TraceCard(
                semanticsLabel: '${alertTypeText(a.type)}. ${a.title}. ${a.body}. ${relativeTime(a.eventTime)}.',
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Row(children: [
                    Pill(alertTypeText(a.type),
                        color: a.type == AlertType.riskEvent ? TraceColors.warning : TraceColors.emerald),
                    const Spacer(),
                    Muted(relativeTime(a.eventTime), size: 12),
                  ]),
                  const SizedBox(height: 8),
                  Text(a.title, style: const TextStyle(fontWeight: FontWeight.w700)),
                  Muted(a.body),
                  if (a.evidenceTx.isNotEmpty) Muted('Evidence transactions: ${a.evidenceTx.length}', size: 12),
                ]),
              ),
            const Disclaimer(),
          ]),
        ),
      ),
    );
  }
}
