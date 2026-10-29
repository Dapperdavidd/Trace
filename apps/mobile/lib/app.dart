import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'api/models.dart';
import 'core/theme.dart';
import 'core/tokens.dart';
import 'screens/alerts_screen.dart';
import 'screens/briefing_screen.dart';
import 'screens/coming_later_screens.dart';
import 'screens/home_screen.dart';
import 'screens/more_screen.dart';
import 'screens/movers_screen.dart';
import 'screens/research_screens.dart';
import 'screens/shadow_screens.dart';
import 'screens/token_screen.dart';
import 'screens/wallet_profile_screen.dart';
import 'screens/watchlist_screen.dart';
import 'screens/welcome_screen.dart';
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
    redirect: (context, state) => redirectFor(ref.read(sessionProvider).phase, state.matchedLocation),
    routes: [
      GoRoute(path: '/splash', builder: (_, __) => const Scaffold(body: Center(child: CircularProgressIndicator()))),
      GoRoute(path: '/welcome', builder: (_, __) => const WelcomeScreen()),
      StatefulShellRoute.indexedStack(
        builder: (context, state, shell) => _Shell(shell),
        branches: [
          StatefulShellBranch(routes: [GoRoute(path: '/home', builder: (_, __) => const HomeScreen())]),
          StatefulShellBranch(routes: [GoRoute(path: '/market', builder: (_, __) => const MoversScreen())]),
          StatefulShellBranch(routes: [GoRoute(path: '/shadow', builder: (_, __) => const ShadowHomeScreen())]),
          StatefulShellBranch(routes: [GoRoute(path: '/watchlist', builder: (_, __) => const WatchlistScreen())]),
          StatefulShellBranch(routes: [GoRoute(path: '/more', builder: (_, __) => const MoreScreen())]),
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
          chain: s.uri.queryParameters['chain'] == null ? null : Chain.parse(s.uri.queryParameters['chain']),
          address: s.uri.queryParameters['address'],
        ),
      ),
      GoRoute(path: '/shadow/results/:id', builder: (_, s) => ShadowResultsScreen(id: s.pathParameters['id'] ?? '')),
      GoRoute(path: '/timeline/:chain/:address', builder: (_, s) => TimelineScreen(chain: chainOf(s), address: addrOf(s))),
      GoRoute(path: '/risk/:chain/:address', builder: (_, s) => RiskRadarScreen(chain: chainOf(s), address: addrOf(s))),
      GoRoute(
        path: '/compare',
        builder: (_, s) => CompareScreen(a: s.uri.queryParameters['a'], b: s.uri.queryParameters['b']),
      ),
      GoRoute(path: '/ask', builder: (_, __) => const AskScreen()),
      GoRoute(path: '/briefing', builder: (_, __) => const BriefingScreen()),
      GoRoute(path: '/wallet', builder: (_, __) => const WalletScreen()),
      GoRoute(path: '/trade', builder: (_, __) => const TradeScreen()),
      GoRoute(path: '/alerts', builder: (_, __) => const AlertsScreen()),
    ],
  );
});

class _Shell extends StatelessWidget {
  const _Shell(this.shell);
  final StatefulNavigationShell shell;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: shell,
      bottomNavigationBar: NavigationBar(
        selectedIndex: shell.currentIndex,
        onDestinationSelected: (i) => shell.goBranch(i, initialLocation: i == shell.currentIndex),
        destinations: const [
          NavigationDestination(icon: Icon(Icons.home_outlined), selectedIcon: Icon(Icons.home), label: 'Home'),
          NavigationDestination(icon: Icon(Icons.bar_chart_outlined), selectedIcon: Icon(Icons.bar_chart), label: 'Market'),
          NavigationDestination(icon: Icon(Icons.science_outlined), selectedIcon: Icon(Icons.science), label: 'Shadow'),
          NavigationDestination(icon: Icon(Icons.visibility_outlined), selectedIcon: Icon(Icons.visibility), label: 'Watchlist'),
          NavigationDestination(icon: Icon(Icons.more_horiz), selectedIcon: Icon(Icons.more_horiz), label: 'More'),
        ],
      ),
    );
  }
}

class TraceApp extends ConsumerWidget {
  const TraceApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MaterialApp.router(
      title: 'TRACE',
      debugShowCheckedModeBanner: false,
      theme: buildTraceTheme(),
      darkTheme: buildTraceTheme(),
      themeMode: ThemeMode.dark,
      routerConfig: ref.watch(routerProvider),
      builder: (context, child) => ColoredBox(color: TraceColors.bg, child: child ?? const SizedBox.shrink()),
    );
  }
}
