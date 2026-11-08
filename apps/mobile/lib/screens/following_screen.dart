import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

class FollowingScreen extends ConsumerWidget {
  const FollowingScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final wallets = ref.watch(watchlistProvider);
    return TraceScaffold(
      title: 'Following',
      subtitle: 'Your coins, wallets and alerts.',
      actions: [
        IconButton(
          tooltip: 'Alerts',
          icon: const Icon(Icons.notifications_none_rounded),
          onPressed: () => context.push('/alerts'),
        ),
      ],
      body: ListView(
        padding: const EdgeInsets.all(TraceSpacing.gutter),
        children: [
          const _SegmentedHeader(),
          const SizedBox(height: 16),
          wallets.when(
            loading: () => const TraceCard(
                child: Center(child: CircularProgressIndicator())),
            error: (e, _) => ErrorState(
                error: e, onRetry: () => ref.invalidate(watchlistProvider)),
            data: (items) => items.isEmpty
                ? const TraceCard(
                    child: EmptyState(
                      title: 'Nothing followed yet',
                      message:
                          'Follow a coin or wallet from Explore to see it here.',
                    ),
                  )
                : Column(children: [
                    for (final item in items)
                      TraceCard(
                        child: Row(children: [
                          const Icon(Icons.account_balance_wallet_outlined,
                              color: TraceColors.emerald),
                          const SizedBox(width: 12),
                          Expanded(child: Text(item.wallet.address)),
                          const Icon(Icons.chevron_right),
                        ]),
                      ),
                  ]),
          ),
          TraceCard(
            onTap: () => context.push('/briefing'),
            child: const Row(children: [
              Icon(Icons.description_outlined, color: TraceColors.emerald),
              SizedBox(width: 12),
              Expanded(
                  child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                    Text('Daily briefing',
                        style: TextStyle(fontWeight: FontWeight.w500)),
                    Muted('Your market summary is ready.', size: 12)
                  ])),
              Icon(Icons.chevron_right),
            ]),
          ),
        ],
      ),
    );
  }
}

class _SegmentedHeader extends StatelessWidget {
  const _SegmentedHeader();
  @override
  Widget build(BuildContext context) => Container(
        padding: const EdgeInsets.all(4),
        decoration: BoxDecoration(
            color: TraceColors.surface,
            borderRadius: BorderRadius.circular(99)),
        child: const Row(children: [
          Expanded(child: Center(child: Text('Coins'))),
          Expanded(child: Center(child: Text('Wallets'))),
          Expanded(child: Center(child: Text('Alerts'))),
        ]),
      );
}
