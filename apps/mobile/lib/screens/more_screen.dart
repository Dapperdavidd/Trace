import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../core/config.dart';
import '../core/copy.dart';
import '../core/tokens.dart';
import '../state/session.dart';
import '../state/providers.dart';
import '../widgets/common.dart';
import 'research_screens.dart';

class MoreScreen extends ConsumerWidget {
  const MoreScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    Widget item(IconData icon, String title, String sub, VoidCallback onTap,
        {bool later = false}) {
      return TraceCard(
        onTap: onTap,
        semanticsLabel: '$title. $sub${later ? '. ${Copy.comingLater}' : ''}',
        child: Row(children: [
          Icon(icon,
              color: later ? TraceColors.textMuted : TraceColors.emerald),
          const SizedBox(width: 12),
          Expanded(
            child:
                Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(title, style: const TextStyle(fontWeight: FontWeight.w700)),
              Muted(sub, size: 12),
            ]),
          ),
          if (later)
            const Pill(Copy.comingLater, color: TraceColors.info)
          else
            const Icon(Icons.chevron_right),
        ]),
      );
    }

    return TraceScaffold(
      title: 'More',
      body: ListView(
          padding: const EdgeInsets.all(TraceSpacing.gutter),
          children: [
            const SectionTitle('Research'),
            item(
                Icons.wb_sunny_outlined,
                'Daily Briefing',
                'A simple summary of what matters',
                () => context.push('/briefing')),
            item(Icons.notifications_none, 'Alerts',
                'Wallet moves and risk changes', () => context.push('/alerts')),
            item(
                Icons.timeline,
                'Opportunity Timeline',
                'What happened to a coin, in order',
                () => pickTokenAndGo(context, ref, '/timeline')),
            item(
                Icons.shield_outlined,
                'Risk Radar',
                'Check a coin for possible risks',
                () => pickTokenAndGo(context, ref, '/risk')),
            item(Icons.compare_arrows, 'Compare Coins',
                'Two coins side by side', () => context.push('/compare')),
            const SectionTitle('Coming later'),
            item(Icons.account_balance_wallet_outlined, 'Wallet',
                'Hold, send and receive crypto', () => context.push('/wallet'),
                later: true),
            item(Icons.swap_horiz, 'Trade & Swap', 'Trade with real money',
                () => context.push('/trade'),
                later: true),
            item(Icons.chat_bubble_outline, 'Ask TRACE',
                'Ask questions in plain words', () => context.push('/ask'),
                later: true),
            const SectionTitle('Account'),
            TraceCard(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(session.phase == SessionPhase.signedIn
                        ? (session.name ?? session.email ?? 'Signed in')
                        : 'Exploring without an account'),
                    const Muted(
                        'Watch-only. TRACE holds no wallet or funds for you.',
                        size: 12),
                    const SizedBox(height: 8),
                    if (session.phase == SessionPhase.signedIn) ...[
                      OutlinedButton(
                        onPressed: () =>
                            ref.read(sessionProvider.notifier).signOut(),
                        child: const Text('Sign out'),
                      ),
                      const SizedBox(height: 8),
                      OutlinedButton(
                        onPressed: () => _confirmDelete(context, ref),
                        child: const Text('Delete my account'),
                      ),
                    ] else
                      OutlinedButton(
                        onPressed: () =>
                            ref.read(sessionProvider.notifier).signOut(),
                        child: const Text('Back to sign in'),
                      ),
                  ]),
            ),
            const SectionTitle('About the data'),
            const _CoverageCard(),
            const TraceCard(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Data status labels',
                        style: TextStyle(fontWeight: FontWeight.w700)),
                    SizedBox(height: 4),
                    Muted(
                        'LIVE: fresh data. STALE: out of date. PARTIAL: some data missing. UNAVAILABLE: source is down. DEMO DATA: fictional example numbers.'),
                    SizedBox(height: 8),
                    Muted(
                        'Observed facts come from on-chain data. Estimates and model assessments are labelled and can be wrong.'),
                  ]),
            ),
            const Muted(
                'API: ${AppConfig.apiUrl}${AppConfig.demoForced ? '  (demo forced)' : ''}',
                size: 11),
            const Disclaimer(),
          ]),
    );
  }

  Future<void> _confirmDelete(BuildContext context, WidgetRef ref) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete your account?'),
        content: const Text(
            'This removes your account and watchlist. It cannot be undone.'),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('Cancel')),
          TextButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('Delete')),
        ],
      ),
    );
    if (ok != true) return;
    try {
      await ref.read(sessionProvider.notifier).deleteAccount();
    } catch (_) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
              content: Text('Could not delete the account. Try again later.')),
        );
      }
    }
  }
}

class _CoverageCard extends ConsumerWidget {
  const _CoverageCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final value = ref.watch(coverageProvider);
    return TraceCard(
      child: AsyncBody(
        value: value,
        onRetry: () => ref.invalidate(coverageProvider),
        builder: (coverage) => Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('Chain and protocol coverage',
                style: TextStyle(fontWeight: FontWeight.w700)),
            const SizedBox(height: 8),
            EnvelopeBar(coverage.env),
            for (final chain in coverage.chains) ...[
              Row(children: [
                Expanded(child: Text(chain.name)),
                Pill(chain.status.toUpperCase(),
                    color:
                        chain.status == 'partial' || chain.status == 'enabled'
                            ? TraceColors.info
                            : TraceColors.textMuted),
              ]),
              for (final protocol in chain.protocols)
                Padding(
                  padding: const EdgeInsets.only(left: 12, top: 4, bottom: 8),
                  child: Muted('${protocol.protocol}: ${protocol.status}'),
                ),
            ],
            Muted(coverage.note, size: 12),
          ],
        ),
      ),
    );
  }
}
