import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/tokens.dart';
import '../state/session.dart';

/// Google-only onboarding, matching the final TRACE visual direction.
class WelcomeScreen extends ConsumerWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final sessionController = ref.read(sessionProvider.notifier);
    final signIn = sessionController.signInWithGoogle;

    return Scaffold(
      backgroundColor: const Color(0xFF050611),
      body: Stack(
        fit: StackFit.expand,
        children: [
          Image.asset(
            'assets/neon-blue-space-horizon.png',
            fit: BoxFit.cover,
            alignment: Alignment.center,
          ),
          SafeArea(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(28, 24, 28, 22),
              child: Column(
                children: [
                  Align(
                    alignment: Alignment.topRight,
                    child: TextButton(
                      onPressed:
                          session.busy ? null : sessionController.explore,
                      style: TextButton.styleFrom(
                        foregroundColor: const Color(0xFFE4E6F0),
                        backgroundColor: const Color(0x66141627),
                        minimumSize: const Size(90, 44),
                        padding: const EdgeInsets.symmetric(horizontal: 18),
                        shape: const StadiumBorder(
                          side: BorderSide(color: Color(0x334C567A)),
                        ),
                      ),
                      child: const Text(
                        'Explore',
                        style: TextStyle(
                          fontSize: 15,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                  ),
                  const Spacer(flex: 4),
                  Image.asset(
                    'assets/trace-ribbon-logo.png',
                    width: 118,
                    height: 118,
                    fit: BoxFit.contain,
                    semanticLabel: 'Trace',
                  ),
                  const SizedBox(height: 28),
                  const _Headline(),
                  const SizedBox(height: 20),
                  const Text(
                    'Trace, analyze, and understand\nonchain activity.',
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      color: Color(0xFFB4B6C2),
                      fontSize: 18,
                      fontWeight: FontWeight.w400,
                      height: 1.45,
                      letterSpacing: -.2,
                    ),
                  ),
                  const Spacer(flex: 13),
                  if (session.error != null)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 12),
                      child: Semantics(
                        liveRegion: true,
                        child: Text(
                          session.error!,
                          textAlign: TextAlign.center,
                          style: const TextStyle(
                            color: Color(0xFFFFA0A7),
                            fontSize: 13,
                            height: 1.35,
                          ),
                        ),
                      ),
                    ),
                  SizedBox(
                    width: double.infinity,
                    height: 62,
                    child: FilledButton(
                      onPressed: session.busy ? null : signIn,
                      style: FilledButton.styleFrom(
                        backgroundColor: Colors.white,
                        disabledBackgroundColor: Colors.white70,
                        foregroundColor: const Color(0xFF101116),
                        shape: const StadiumBorder(),
                        elevation: 0,
                      ),
                      child: session.busy
                          ? const SizedBox.square(
                              dimension: 22,
                              child: CircularProgressIndicator(
                                strokeWidth: 2.5,
                                color: Color(0xFF2563EB),
                              ),
                            )
                          : const Row(
                              mainAxisAlignment: MainAxisAlignment.center,
                              children: [
                                Image(
                                  image: AssetImage('assets/google-g.png'),
                                  width: 26,
                                  height: 26,
                                  fit: BoxFit.contain,
                                  semanticLabel: 'Google',
                                ),
                                SizedBox(width: 14),
                                Text(
                                  'Sign in with Google',
                                  style: TextStyle(
                                    fontSize: 18,
                                    fontWeight: FontWeight.w600,
                                    letterSpacing: -.25,
                                  ),
                                ),
                              ],
                            ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Headline extends StatelessWidget {
  const _Headline();

  @override
  Widget build(BuildContext context) => const Text.rich(
        TextSpan(
          children: [
            TextSpan(text: 'Look inside\nthe chain'),
            TextSpan(
              text: '.',
              style: TextStyle(color: Color(0xFF3468FF)),
            ),
          ],
        ),
        textAlign: TextAlign.center,
        style: TextStyle(
          color: TraceColors.text,
          fontSize: 43,
          fontWeight: FontWeight.w600,
          height: 1,
          letterSpacing: -2,
        ),
      );
}
