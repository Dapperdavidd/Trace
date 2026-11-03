import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/models.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../state/session.dart';
import '../widgets/common.dart';

class AccountSessionsScreen extends ConsumerStatefulWidget {
  const AccountSessionsScreen({super.key});

  @override
  ConsumerState<AccountSessionsScreen> createState() =>
      _AccountSessionsScreenState();
}

class _AccountSessionsScreenState extends ConsumerState<AccountSessionsScreen> {
  late Future<List<AccountSession>> _sessions = _load();
  String? _busyId;
  Object? _actionError;

  Future<List<AccountSession>> _load() => ref.read(apiProvider).sessions();

  void _reload() {
    setState(() {
      _actionError = null;
      _sessions = _load();
    });
  }

  Future<void> _revoke(String id) async {
    setState(() {
      _busyId = id;
      _actionError = null;
    });
    try {
      await ref.read(apiProvider).revokeSession(id);
      _reload();
    } catch (error) {
      setState(() => _actionError = error);
    } finally {
      if (mounted) setState(() => _busyId = null);
    }
  }

  Future<void> _signOutAll() async {
    setState(() {
      _busyId = 'all';
      _actionError = null;
    });
    try {
      await ref.read(sessionProvider.notifier).signOutAll();
    } catch (error) {
      if (mounted) setState(() => _actionError = error);
    } finally {
      if (mounted) setState(() => _busyId = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    return TraceScaffold(
      title: 'Signed-in devices',
      subtitle: 'Review active TRACE account sessions',
      body: ListView(
        padding: const EdgeInsets.all(TraceSpacing.gutter),
        children: [
          FutureBuilder<List<AccountSession>>(
            future: _sessions,
            builder: (context, snapshot) {
              if (snapshot.connectionState == ConnectionState.waiting) {
                return const Center(
                    child: Padding(
                        padding: EdgeInsets.all(32),
                        child: CircularProgressIndicator()));
              }
              if (snapshot.hasError) {
                return ErrorState(error: snapshot.error!, onRetry: _reload);
              }
              final sessions = snapshot.data ?? const <AccountSession>[];
              if (sessions.isEmpty) {
                return const EmptyState(
                  title: 'No active sessions',
                  message: 'Sign in again to create a new session.',
                );
              }
              return Column(
                children: [
                  for (final session in sessions)
                    TraceCard(
                      child: Row(
                        children: [
                          const Icon(Icons.devices_outlined,
                              color: TraceColors.emerald),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Wrap(
                                  spacing: 8,
                                  runSpacing: 4,
                                  crossAxisAlignment: WrapCrossAlignment.center,
                                  children: [
                                    Text(session.deviceLabel,
                                        style: const TextStyle(
                                            fontWeight: FontWeight.w700)),
                                    if (session.current)
                                      const Pill('This device',
                                          color: TraceColors.positive),
                                  ],
                                ),
                                Muted(
                                    'Signed in ${relativeTime(session.createdAt)}',
                                    size: 12),
                                Muted(
                                    'Expires ${relativeTime(session.expiresAt)}',
                                    size: 12),
                              ],
                            ),
                          ),
                          if (!session.current)
                            OutlinedButton(
                              onPressed: _busyId == null
                                  ? () => _revoke(session.id)
                                  : null,
                              child: Text(_busyId == session.id
                                  ? 'Revoking...'
                                  : 'Revoke'),
                            ),
                        ],
                      ),
                    ),
                ],
              );
            },
          ),
          if (_actionError != null)
            ErrorState(error: _actionError!, onRetry: _reload),
          const SectionTitle('Sign out everywhere'),
          const Muted(
              'This immediately revokes every active session, including this device.'),
          const SizedBox(height: 8),
          OutlinedButton(
            onPressed: _busyId == null ? _signOutAll : null,
            child: Text(_busyId == 'all'
                ? 'Signing out...'
                : 'Sign out on all devices'),
          ),
        ],
      ),
    );
  }
}
