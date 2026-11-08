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
    final signIn = ref.read(sessionProvider.notifier).signInWithGoogle;

    return Scaffold(
      backgroundColor: const Color(0xFF050611),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final height = constraints.maxHeight;
          return Stack(
            fit: StackFit.expand,
            children: [
              const _Atmosphere(),
              Positioned(
                left: -height * .34,
                right: -height * .34,
                top: height * .60,
                height: height * .57,
                child: const _BlueHorizon(),
              ),
              SafeArea(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(28, 24, 28, 22),
                  child: Column(
                    children: [
                      const Spacer(flex: 5),
                      Image.asset(
                        'assets/trace-ribbon-logo.png',
                        width: 150,
                        height: 150,
                        fit: BoxFit.contain,
                        semanticLabel: 'Trace',
                      ),
                      const SizedBox(height: 38),
                      const _Headline(),
                      const SizedBox(height: 24),
                      const Text(
                        'Trace, analyze, and understand\nonchain activity.',
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          color: Color(0xFFB4B6C2),
                          fontSize: 20,
                          fontWeight: FontWeight.w400,
                          height: 1.45,
                          letterSpacing: -.2,
                        ),
                      ),
                      const Spacer(flex: 12),
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
                                    _GoogleMark(),
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
          );
        },
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
          fontSize: 50,
          fontWeight: FontWeight.w600,
          height: .98,
          letterSpacing: -2.4,
        ),
      );
}

class _Atmosphere extends StatelessWidget {
  const _Atmosphere();

  @override
  Widget build(BuildContext context) => const DecoratedBox(
        decoration: BoxDecoration(
          gradient: RadialGradient(
            center: Alignment(.42, -.34),
            radius: 1.05,
            colors: [Color(0xFF111329), Color(0xFF070812), Color(0xFF03040A)],
            stops: [0, .46, 1],
          ),
        ),
      );
}

class _BlueHorizon extends StatelessWidget {
  const _BlueHorizon();

  @override
  Widget build(BuildContext context) => Container(
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          gradient: const RadialGradient(
            center: Alignment(0, -.74),
            radius: .78,
            colors: [Color(0xFF243170), Color(0xFF0E132B), Color(0xFF050611)],
            stops: [0, .48, 1],
          ),
          border: Border.all(color: const Color(0xFF5E83FF), width: 1.2),
          boxShadow: const [
            BoxShadow(
                color: Color(0xCC123CFF), blurRadius: 46, spreadRadius: 3),
            BoxShadow(
                color: Color(0x993373FF), blurRadius: 95, spreadRadius: 10),
          ],
        ),
      );
}

class _GoogleMark extends StatelessWidget {
  const _GoogleMark();

  @override
  Widget build(BuildContext context) => const Text(
        'G',
        style: TextStyle(
          color: Color(0xFF4285F4),
          fontSize: 26,
          fontWeight: FontWeight.w700,
          fontFamily: 'Arial',
        ),
      );
}
