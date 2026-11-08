import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/config.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../state/session.dart';
import '../widgets/block_field.dart';
import '../widgets/common.dart';

/// Onboarding in the v11 design: the block field behind one headline, then a passkey.
class WelcomeScreen extends ConsumerStatefulWidget {
  const WelcomeScreen({super.key});

  @override
  ConsumerState<WelcomeScreen> createState() => _WelcomeScreenState();
}

enum _Mode { start, create, signIn }

class _WelcomeScreenState extends ConsumerState<WelcomeScreen> {
  _Mode mode = _Mode.start;
  final username = TextEditingController();
  final displayName = TextEditingController();

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
    final height = MediaQuery.of(context).size.height;
    return Scaffold(
      body: Stack(
        children: [
          Positioned(
            left: 0,
            right: 0,
            top: 0,
            height: height * 0.56,
            child: const BlockField(),
          ),
          SafeArea(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(22, 12, 14, 0),
              child: Row(
                children: [
                  Semantics(
                    header: true,
                    label: 'trace',
                    child: const ExcludeSemantics(child: _Logo()),
                  ),
                  const Spacer(),
                  TextButton(
                    onPressed: notifier.explore,
                    style: TextButton.styleFrom(
                      backgroundColor: TraceColors.surface,
                      foregroundColor: TraceColors.textMuted,
                      minimumSize: const Size(0, 40),
                      padding: const EdgeInsets.symmetric(horizontal: 16),
                    ),
                    child: const Text('Explore first'),
                  ),
                ],
              ),
            ),
          ),
          Align(
            alignment: Alignment.bottomCenter,
            child: Container(
              constraints: BoxConstraints(
                  minHeight: height * 0.46, maxHeight: height * 0.9),
              width: double.infinity,
              decoration: const BoxDecoration(
                color: TraceColors.bg,
                borderRadius: BorderRadius.vertical(top: Radius.circular(30)),
              ),
              child: SafeArea(
                top: false,
                child: SingleChildScrollView(
                  padding: const EdgeInsets.fromLTRB(22, 22, 22, 24),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      ValueListenableBuilder<bool>(
                        valueListenable: ref.watch(demoActiveProvider),
                        builder: (_, active, __) => active
                            ? const Padding(
                                padding: EdgeInsets.only(bottom: 12),
                                child: DemoBanner())
                            : const SizedBox.shrink(),
                      ),
                      _GreenPill(mode == _Mode.start
                          ? 'On-chain research, made simple'
                          : mode == _Mode.create
                              ? 'No password. Just a passkey.'
                              : 'Welcome back'),
                      const SizedBox(height: 14),
                      _Headline(
                        mode == _Mode.start
                            ? "See who's buying"
                            : mode == _Mode.create
                                ? 'Create your account'
                                : 'Sign in',
                        mode == _Mode.start
                            ? 'before you buy.'
                            : mode == _Mode.create
                                ? 'in about 20 seconds.'
                                : 'with your passkey.',
                      ),
                      const SizedBox(height: 12),
                      if (mode == _Mode.start) ...[
                        const Text(
                          'trace reads every trade on the chain and tells you who is buying, in plain words.',
                          style: TextStyle(
                              color: TraceColors.textMuted,
                              fontSize: 15,
                              height: 1.45),
                        ),
                        const SizedBox(height: 22),
                        _wide(FilledButton(
                          onPressed: () =>
                              setState(() => mode = _Mode.create),
                          child: const Text('Create an account'),
                        )),
                        const SizedBox(height: 8),
                        _wide(OutlinedButton(
                          onPressed: () =>
                              setState(() => mode = _Mode.signIn),
                          child: const Text('I already have one'),
                        )),
                      ] else
                        ..._form(session, notifier),
                      if (session.error != null)
                        Padding(
                          padding: const EdgeInsets.only(top: 12),
                          child: Semantics(
                            liveRegion: true,
                            child: Container(
                              width: double.infinity,
                              padding: const EdgeInsets.all(12),
                              decoration: BoxDecoration(
                                color: TraceColors.surfaceRaised,
                                borderRadius: BorderRadius.circular(16),
                              ),
                              child: Text(session.error!),
                            ),
                          ),
                        ),
                      const SizedBox(height: 14),
                      const Text(
                        'trace never holds your keys or funds. Your NEAR wallet signs every copy limit.',
                        style:
                            TextStyle(color: TraceColors.textFaint, fontSize: 12.5),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _wide(Widget child) =>
      SizedBox(width: double.infinity, height: 52, child: child);

  List<Widget> _form(SessionState session, SessionController notifier) {
    final creating = mode == _Mode.create;
    return [
      if (creating) ...[
        TextField(
          controller: displayName,
          textInputAction: TextInputAction.next,
          autofillHints: const [AutofillHints.name],
          maxLength: 80,
          decoration: const InputDecoration(
              hintText: 'Your name', counterText: ''),
        ),
        const SizedBox(height: 10),
      ],
      TextField(
        controller: username,
        autocorrect: false,
        enableSuggestions: false,
        textCapitalization: TextCapitalization.none,
        autofillHints: const [AutofillHints.username],
        decoration: const InputDecoration(hintText: 'Username'),
      ),
      const SizedBox(height: 14),
      _wide(FilledButton.icon(
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
                ? 'Create with passkey'
                : 'Sign in with passkey'),
      )),
      const SizedBox(height: 8),
      _wide(OutlinedButton(
        onPressed: session.busy
            ? null
            : () => setState(
                () => mode = creating ? _Mode.signIn : _Mode.create),
        child: Text(creating
            ? 'I already have an account'
            : 'Create an account instead'),
      )),
      if (AppConfig.googleConfigured) ...[
        const SizedBox(height: 8),
        _wide(OutlinedButton.icon(
          onPressed: session.busy ? null : notifier.signInWithGoogle,
          icon: const Icon(Icons.login),
          label: const Text('Sign in with Google'),
        )),
      ],
      if (AppConfig.devAuth) ...[
        const SizedBox(height: 8),
        _wide(OutlinedButton.icon(
          onPressed: session.busy ? null : notifier.signInForDevelopment,
          icon: const Icon(Icons.developer_mode),
          label: const Text('Local development sign-in'),
        )),
      ],
      const SizedBox(height: 10),
      const Text(
        'Your face or fingerprint stays on your device. trace stores only a public key.',
        style: TextStyle(color: TraceColors.textMuted, fontSize: 13),
      ),
    ];
  }
}

class _Logo extends StatelessWidget {
  const _Logo();
  @override
  Widget build(BuildContext context) => const Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.manage_search_rounded,
              color: TraceColors.text, size: 24),
          SizedBox(width: 8),
          Text('trace',
              style: TextStyle(
                  fontSize: 20,
                  fontWeight: FontWeight.w600,
                  letterSpacing: -0.6)),
        ],
      );
}

class _GreenPill extends StatelessWidget {
  const _GreenPill(this.text);
  final String text;
  @override
  Widget build(BuildContext context) => Container(
        padding: const EdgeInsets.fromLTRB(8, 5, 12, 5),
        decoration: BoxDecoration(
          color: TraceColors.surface,
          borderRadius: BorderRadius.circular(99),
        ),
        child: Row(mainAxisSize: MainAxisSize.min, children: [
          Container(
              width: 7,
              height: 7,
              decoration: const BoxDecoration(
                  color: TraceColors.emerald, shape: BoxShape.circle)),
          const SizedBox(width: 7),
          Text(text,
              style: const TextStyle(
                  fontSize: 12.5, color: TraceColors.textMuted)),
        ]),
      );
}

class _Headline extends StatelessWidget {
  const _Headline(this.first, this.second);
  final String first;
  final String second;
  @override
  Widget build(BuildContext context) => Semantics(
        header: true,
        child: Text.rich(
          TextSpan(children: [
            TextSpan(text: '$first\n'),
            TextSpan(
                text: second,
                style: const TextStyle(color: TraceColors.emerald)),
          ]),
          style: const TextStyle(
              fontSize: 32,
              fontWeight: FontWeight.w500,
              letterSpacing: -1.4,
              height: 1.05),
        ),
      );
}
