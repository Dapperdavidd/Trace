import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/config.dart';
import '../core/copy.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../state/session.dart';
import '../widgets/common.dart';

class WelcomeScreen extends ConsumerWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final notifier = ref.read(sessionProvider.notifier);
    final configured = AppConfig.googleConfigured;
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            ValueListenableBuilder<bool>(
              valueListenable: ref.watch(demoActiveProvider),
              builder: (_, active, __) => active ? const DemoBanner() : const SizedBox.shrink(),
            ),
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(TraceSpacing.gutter),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const SizedBox(height: 32),
                    Semantics(
                      header: true,
                      label: 'trace',
                      child: const ExcludeSemantics(
                        child: Text.rich(TextSpan(children: [
                          TextSpan(text: 'trace', style: TextStyle(fontSize: 56, fontWeight: FontWeight.w900)),
                          TextSpan(text: '.', style: TextStyle(fontSize: 56, fontWeight: FontWeight.w900, color: TraceColors.emerald)),
                        ])),
                      ),
                    ),
                    const SizedBox(height: 16),
                    const Text('Find smart wallets and moving coins, and understand why.',
                        style: TextStyle(fontSize: 26, fontWeight: FontWeight.w800, height: 1.2)),
                    const SizedBox(height: 12),
                    const Muted('Discover opportunities. Watch any wallet. Practice with paper trades before you risk real money.', size: 16),
                    const SizedBox(height: 24),
                    const TraceCard(
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Icon(Icons.visibility_outlined, color: TraceColors.emerald),
                          SizedBox(width: 12),
                          Expanded(child: Text(Copy.watchOnly)),
                        ],
                      ),
                    ),
                    const SizedBox(height: 8),
                    SizedBox(
                      width: double.infinity,
                      child: FilledButton.icon(
                        onPressed: session.busy || !configured ? null : notifier.signInWithGoogle,
                        icon: const Icon(Icons.login),
                        label: Text(session.busy ? 'Signing in...' : 'Sign in with Google'),
                      ),
                    ),
                    if (!configured)
                      const Padding(
                        padding: EdgeInsets.only(top: 8),
                        child: Muted(
                            'Google Sign-In is not configured in this build (missing TRACE_GOOGLE_CLIENT_ID). You can still explore.'),
                      ),
                    if (session.error != null)
                      Padding(
                        padding: const EdgeInsets.only(top: 8),
                        child: Semantics(
                          liveRegion: true,
                          child: Text(session.error!, style: const TextStyle(color: TraceColors.negative)),
                        ),
                      ),
                    const SizedBox(height: 12),
                    SizedBox(
                      width: double.infinity,
                      child: OutlinedButton(
                        onPressed: notifier.explore,
                        child: const Text('Explore without signing in'),
                      ),
                    ),
                    const SizedBox(height: 12),
                    SizedBox(
                      width: double.infinity,
                      child: OutlinedButton(
                        onPressed: null,
                        child: Text('Create wallet (${Copy.comingLater})'),
                      ),
                    ),
                    const SizedBox(height: 4),
                    const Muted('Creating or connecting a spendable wallet is not available yet. Signing in never gives TRACE access to funds.', size: 12),
                    const Disclaimer(),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
