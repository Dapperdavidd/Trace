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
              builder: (_, active, __) =>
                  active ? const DemoBanner() : const SizedBox.shrink(),
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
                      label: 'TRACE',
                      child: const ExcludeSemantics(
                        child: Text('TRACE',
                            style: TextStyle(
                                fontSize: 52,
                                fontWeight: FontWeight.w900,
                                color: TraceColors.text)),
                      ),
                    ),
                    const SizedBox(height: 16),
                    const Text(
                        'Find smart wallets and moving coins, and understand why.',
                        style: TextStyle(
                            fontSize: 26,
                            fontWeight: FontWeight.w800,
                            height: 1.2)),
                    const SizedBox(height: 12),
                    const Muted(
                        'Discover opportunities. Watch any wallet. Practice with paper trades before you risk real money.',
                        size: 16),
                    const SizedBox(height: 24),
                    const TraceCard(
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Icon(Icons.visibility_outlined,
                              color: TraceColors.emerald),
                          SizedBox(width: 12),
                          Expanded(child: Text(Copy.watchOnly)),
                        ],
                      ),
                    ),
                    const SizedBox(height: 8),
                    const _PasskeyPanel(),
                    const Padding(
                      padding: EdgeInsets.symmetric(vertical: 8),
                      child: Row(children: [
                        Expanded(child: Divider()),
                        Padding(
                          padding: EdgeInsets.symmetric(horizontal: 12),
                          child: Muted('Alternative sign-in', size: 12),
                        ),
                        Expanded(child: Divider()),
                      ]),
                    ),
                    SizedBox(
                      width: double.infinity,
                      child: FilledButton.icon(
                        onPressed: session.busy || !configured
                            ? null
                            : notifier.signInWithGoogle,
                        icon: const Icon(Icons.login),
                        label: Text(session.busy
                            ? 'Signing in...'
                            : 'Sign in with Google'),
                      ),
                    ),
                    if (!configured)
                      const Padding(
                        padding: EdgeInsets.only(top: 8),
                        child: Muted(
                            'Google Sign-In is not configured in this build (missing TRACE_GOOGLE_CLIENT_ID). You can still explore.'),
                      ),
                    if (AppConfig.devAuth) ...[
                      const SizedBox(height: 12),
                      SizedBox(
                        width: double.infinity,
                        child: OutlinedButton.icon(
                          onPressed: session.busy
                              ? null
                              : notifier.signInForDevelopment,
                          icon: const Icon(Icons.developer_mode),
                          label: const Text('Local development sign-in'),
                        ),
                      ),
                      const Muted(
                          'Shown only in builds started with TRACE_DEV_AUTH=true.',
                          size: 12),
                    ],
                    if (session.error != null)
                      Padding(
                        padding: const EdgeInsets.only(top: 8),
                        child: Semantics(
                          liveRegion: true,
                          child: Text(session.error!,
                              style:
                                  const TextStyle(color: TraceColors.negative)),
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
                    const SizedBox(
                      width: double.infinity,
                      child: OutlinedButton(
                        onPressed: null,
                        child: Text('Create wallet (${Copy.comingLater})'),
                      ),
                    ),
                    const SizedBox(height: 4),
                    const Muted(
                        'Creating or connecting a spendable wallet is not available yet. A passkey signs you into TRACE but never gives TRACE access to funds.',
                        size: 12),
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

class _PasskeyPanel extends ConsumerStatefulWidget {
  const _PasskeyPanel();

  @override
  ConsumerState<_PasskeyPanel> createState() => _PasskeyPanelState();
}

class _PasskeyPanelState extends ConsumerState<_PasskeyPanel> {
  final username = TextEditingController();
  final displayName = TextEditingController();
  bool creating = false;

  @override
  void dispose() {
    username.dispose();
    displayName.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final session = ref.watch(sessionProvider);
    final notifier = ref.read(sessionProvider.notifier);
    return TraceCard(
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        const Row(children: [
          Icon(Icons.fingerprint, color: TraceColors.emerald),
          SizedBox(width: 10),
          Expanded(
            child: Text('Passkey',
                style: TextStyle(fontSize: 18, fontWeight: FontWeight.w800)),
          ),
          Pill('PRIMARY'),
        ]),
        const SizedBox(height: 8),
        const Muted(
            'Use Face ID, Touch ID, fingerprint or your device PIN. Biometrics never leave your device.'),
        const SizedBox(height: 12),
        SegmentedButton<bool>(
          segments: const [
            ButtonSegment(value: false, label: Text('Sign in')),
            ButtonSegment(value: true, label: Text('Create account')),
          ],
          selected: {creating},
          onSelectionChanged: session.busy
              ? null
              : (value) => setState(() => creating = value.first),
        ),
        const SizedBox(height: 12),
        if (creating) ...[
          TextField(
            controller: displayName,
            textInputAction: TextInputAction.next,
            autofillHints: const [AutofillHints.name],
            maxLength: 80,
            decoration: const InputDecoration(labelText: 'Display name'),
          ),
          const SizedBox(height: 8),
        ],
        TextField(
          controller: username,
          autocorrect: false,
          enableSuggestions: false,
          textCapitalization: TextCapitalization.none,
          autofillHints: const [AutofillHints.username],
          decoration: const InputDecoration(labelText: 'Username'),
        ),
        const SizedBox(height: 12),
        SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: session.busy
                ? null
                : () {
                    final handle = username.text.trim().toLowerCase();
                    if (creating) {
                      notifier.registerPasskey(handle, displayName.text.trim());
                    } else {
                      notifier.signInWithPasskey(handle);
                    }
                  },
            icon: const Icon(Icons.fingerprint),
            label: Text(session.busy
                ? 'Waiting for your device'
                : creating
                    ? 'Create account with passkey'
                    : 'Sign in with passkey'),
          ),
        ),
      ]),
    );
  }
}
