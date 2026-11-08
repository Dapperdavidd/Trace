import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'api/models.dart';
import 'core/theme.dart';
import 'core/tokens.dart';
import 'screens/alerts_screen.dart';
import 'screens/account_sessions_screen.dart';
import 'screens/briefing_screen.dart';
import 'screens/home_screen.dart';
import 'screens/near_screen.dart';
import 'screens/more_screen.dart';
import 'screens/movers_screen.dart';
import 'screens/research_screens.dart';
import 'screens/shadow_screens.dart';
import 'screens/token_screen.dart';
import 'screens/wallet_profile_screen.dart';
import 'screens/watchlist_screen.dart';
import 'screens/welcome_screen.dart';
import 'screens/following_screen.dart';
import 'screens/you_screen.dart';
import 'state/session.dart';

class _RouterRefresh extends ChangeNotifier {
  void ping() => notifyListeners();
}

/// Pure routing decision, kept separate so it can be reasoned about and tested.
String? redirectFor(SessionPhase phase, String location) {
  switch (phase) {
    case SessionPhase.loading:
      return location == '/splash' ? null : '/splash';
    case SessionPhase.signedOut:
      return location == '/welcome' ? null : '/welcome';
    case SessionPhase.explorer:
    case SessionPhase.signedIn:
      return (location == '/welcome' || location == '/splash') ? '/home' : null;
  }
}

final routerProvider = Provider<GoRouter>((ref) {
  final refresh = _RouterRefresh();
  ref.listen<SessionState>(sessionProvider, (_, __) => refresh.ping());
  ref.onDispose(refresh.dispose);

  Chain chainOf(GoRouterState s) => Chain.parse(s.pathParameters['chain']);
  String addrOf(GoRouterState s) => s.pathParameters['address'] ?? '';

  return GoRouter(
    initialLocation: '/home',
    refreshListenable: refresh,
    redirect: (context, state) {
      final phase = ref.read(sessionProvider).phase;
      final loc = state.matchedLocation;
      // Keep the screen someone asked for (a deep link or --route) across the splash.
      if (phase == SessionPhase.loading && loc != '/splash') {
        return '/splash?next=${Uri.encodeComponent(state.uri.toString())}';
      }
      final next = state.uri.queryParameters['next'];
      if (loc == '/splash' &&
          next != null &&
          next.startsWith('/') &&
          (phase == SessionPhase.explorer || phase == SessionPhase.signedIn)) {
        return next;
      }
      return redirectFor(phase, loc);
    },
    routes: [
      GoRoute(
          path: '/splash',
          builder: (_, __) =>
              const Scaffold(body: Center(child: CircularProgressIndicator()))),
      GoRoute(path: '/welcome', builder: (_, __) => const WelcomeScreen()),
      StatefulShellRoute.indexedStack(
        builder: (context, state, shell) => _Shell(shell),
        branches: [
          StatefulShellBranch(routes: [
            GoRoute(path: '/home', builder: (_, __) => const HomeScreen())
          ]),
          StatefulShellBranch(routes: [
            GoRoute(
                path: '/following', builder: (_, __) => const FollowingScreen())
          ]),
          StatefulShellBranch(routes: [
            GoRoute(path: '/you', builder: (_, __) => const YouScreen())
          ]),
        ],
      ),
      GoRoute(
        path: '/token/:chain/:address',
        builder: (_, s) => TokenScreen(chain: chainOf(s), address: addrOf(s)),
      ),
      GoRoute(
        path: '/wallet-profile/:chain/:address',
        builder: (_, s) => WalletProfileScreen(
          chain: chainOf(s),
          address: addrOf(s),
          summary: s.extra is WalletSummary ? s.extra as WalletSummary : null,
        ),
      ),
      GoRoute(
        path: '/shadow/setup',
        builder: (_, s) => ShadowSetupScreen(
          chain: s.uri.queryParameters['chain'] == null
              ? null
              : Chain.parse(s.uri.queryParameters['chain']),
          address: s.uri.queryParameters['address'],
        ),
      ),
      GoRoute(
          path: '/shadow/results/:id',
          builder: (_, s) =>
              ShadowResultsScreen(id: s.pathParameters['id'] ?? '')),
      GoRoute(
          path: '/timeline/:chain/:address',
          builder: (_, s) =>
              TimelineScreen(chain: chainOf(s), address: addrOf(s))),
      GoRoute(
          path: '/risk/:chain/:address',
          builder: (_, s) =>
              RiskRadarScreen(chain: chainOf(s), address: addrOf(s))),
      GoRoute(
        path: '/compare',
        builder: (_, s) => CompareScreen(
            a: s.uri.queryParameters['a'], b: s.uri.queryParameters['b']),
      ),
      GoRoute(path: '/shadow', builder: (_, __) => const ShadowHomeScreen()),
      GoRoute(path: '/watchlist', builder: (_, __) => const WatchlistScreen()),
      GoRoute(path: '/more', builder: (_, __) => const MoreScreen()),
      GoRoute(path: '/briefing', builder: (_, __) => const BriefingScreen()),
      GoRoute(path: '/movers', builder: (_, __) => const MoversScreen()),
      GoRoute(path: '/alerts', builder: (_, __) => const AlertsScreen()),
      GoRoute(path: '/near', builder: (_, __) => const NearScreen()),
      GoRoute(
          path: '/account-sessions',
          builder: (_, __) => const AccountSessionsScreen()),
    ],
  );
});

class _Shell extends StatelessWidget {
  const _Shell(this.shell);
  final StatefulNavigationShell shell;

  @override
  Widget build(BuildContext context) {
    void go(int i) =>
        shell.goBranch(i, initialLocation: i == shell.currentIndex);
    return Scaffold(
      extendBody: true,
      body: shell,
      bottomNavigationBar: _PillTabs(index: shell.currentIndex, onTap: go),
    );
  }
}

/// Minimal three-tab navigation: discovery, following, and account/security.
class _PillTabs extends StatelessWidget {
  const _PillTabs({required this.index, required this.onTap});
  final int index;
  final ValueChanged<int> onTap;

  static const _items = [
    (Icons.explore_outlined, 'Explore'),
    (Icons.bookmark_border_rounded, 'Following'),
    (Icons.person_outline_rounded, 'You'),
  ];

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      top: false,
      minimum: const EdgeInsets.fromLTRB(14, 0, 14, 16),
      child: Container(
        height: 66,
        padding: const EdgeInsets.symmetric(horizontal: 8),
        decoration: BoxDecoration(
          color: TraceColors.surfaceRaised,
          borderRadius: BorderRadius.circular(99),
          boxShadow: const [
            BoxShadow(color: TraceColors.bg, spreadRadius: 6, blurRadius: 0)
          ],
        ),
        child: Row(
          children: [
            for (var i = 0; i < _items.length; i++)
              Expanded(child: _tab(i, _items[i].$1, _items[i].$2)),
          ],
        ),
      ),
    );
  }

  Widget _tab(int i, IconData icon, String label) {
    final on = index == i;
    return Semantics(
      button: true,
      selected: on,
      label: label,
      excludeSemantics: true,
      child: InkResponse(
        onTap: () => onTap(i),
        radius: 30,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon,
                size: 21,
                color: on ? TraceColors.emerald : TraceColors.textFaint),
            const SizedBox(height: 3),
            Text(label,
                style: TextStyle(
                    fontSize: 10.5,
                    fontWeight: FontWeight.w500,
                    color: on ? TraceColors.text : TraceColors.textFaint)),
          ],
        ),
      ),
    );
  }
}

class TraceApp extends ConsumerWidget {
  const TraceApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MaterialApp.router(
      title: 'trace',
      debugShowCheckedModeBanner: false,
      theme: buildTraceTheme(),
      darkTheme: buildTraceTheme(),
      themeMode: ThemeMode.dark,
      routerConfig: ref.watch(routerProvider),
      builder: (context, child) => ColoredBox(
          color: TraceColors.bg, child: child ?? const SizedBox.shrink()),
    );
  }
}
