import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/api_error.dart';
import '../api/models.dart';
import '../core/copy.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import '../core/validation.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

String watchStatusText(WatchStatus s) => switch (s) {
      WatchStatus.active => 'Active',
      WatchStatus.paused => 'Paused',
      WatchStatus.partialMonitoring => 'Partly monitored',
      WatchStatus.unsupportedProtocol => 'Some activity not supported',
      WatchStatus.waitingForActivity => 'Waiting for activity',
      WatchStatus.unknown => 'Unknown',
    };

String watchStatusHelp(WatchStatus s) => switch (s) {
      WatchStatus.active => 'GiFTED! is checking this wallet.',
      WatchStatus.paused => 'Checks are paused.',
      WatchStatus.partialMonitoring =>
        'GiFTED! can see only part of this wallet\'s activity.',
      WatchStatus.unsupportedProtocol =>
        'This wallet uses apps GiFTED! does not read yet.',
      WatchStatus.waitingForActivity =>
        'No trades seen yet. We will tell you when one appears.',
      WatchStatus.unknown => 'Status not reported.',
    };

class WatchlistScreen extends ConsumerWidget {
  const WatchlistScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(watchlistProvider);
    return TraceScaffold(
      title: 'Watchlist',
      subtitle: 'Public wallets you follow. Watch-only.',
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => showAddWatchSheet(context, ref),
        icon: const Icon(Icons.add),
        label: const Text('Add address'),
      ),
      body: AsyncBody<List<WatchlistItem>>(
        value: v,
        onRetry: () => ref.invalidate(watchlistProvider),
        builder: (items) => RefreshIndicator(
          onRefresh: () async {
            final refreshed = ref.refresh(watchlistProvider.future);
            await refreshed;
          },
          child: ListView(
            padding: const EdgeInsets.fromLTRB(TraceSpacing.gutter,
                TraceSpacing.gutter, TraceSpacing.gutter, 96),
            children: [
              const Muted(Copy.watchOnly, size: 12),
              const SizedBox(height: 12),
              if (items.isEmpty)
                const EmptyState(
                  title: 'No wallets yet',
                  message:
                      'Add a public wallet address to follow its trades. You do not need keys or a connected wallet.',
                ),
              for (final i in items) _WatchTile(i),
            ],
          ),
        ),
      ),
    );
  }
}

class _WatchTile extends ConsumerWidget {
  const _WatchTile(this.item);
  final WatchlistItem item;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final w = item.wallet;
    return TraceCard(
      onTap: () => context.push(w.path),
      child: Row(children: [
        Expanded(
          child:
              Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(w.displayName,
                style:
                    const TextStyle(fontWeight: FontWeight.w800, fontSize: 16)),
            Muted('${w.chain.label}  ${shortAddress(w.address)}', size: 12),
            const SizedBox(height: 6),
            Pill(watchStatusText(item.status),
                color: item.status == WatchStatus.active
                    ? TraceColors.positive
                    : TraceColors.warning),
            Muted(watchStatusHelp(item.status), size: 12),
            Muted('Last checked ${relativeTime(item.lastCheckedAt)}', size: 12),
          ]),
        ),
        IconButton(
          tooltip: 'Remove ${w.displayName} from watchlist',
          icon: const Icon(Icons.delete_outline),
          onPressed: () async {
            final ok = await showDialog<bool>(
              context: context,
              builder: (ctx) => AlertDialog(
                title: const Text('Remove wallet?'),
                content: Text('Stop watching ${w.displayName}?'),
                actions: [
                  TextButton(
                      onPressed: () => Navigator.pop(ctx, false),
                      child: const Text('Cancel')),
                  TextButton(
                      onPressed: () => Navigator.pop(ctx, true),
                      child: const Text('Remove')),
                ],
              ),
            );
            if (ok == true) {
              try {
                await ref.read(apiProvider).removeWatch(item.id);
              } catch (_) {
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                        content: Text('Could not remove this wallet.')),
                  );
                }
              }
              ref.invalidate(watchlistProvider);
            }
          },
        ),
      ]),
    );
  }
}

Future<void> showAddWatchSheet(BuildContext context, WidgetRef ref) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    backgroundColor: TraceColors.surface,
    builder: (ctx) => Padding(
      padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(ctx).bottom),
      child: const _AddWatchForm(),
    ),
  );
}

class _AddWatchForm extends ConsumerStatefulWidget {
  const _AddWatchForm();
  @override
  ConsumerState<_AddWatchForm> createState() => _AddWatchFormState();
}

class _AddWatchFormState extends ConsumerState<_AddWatchForm> {
  Chain _chain = Chain.solana;
  final _addr = TextEditingController();
  final _alias = TextEditingController();
  String? _error;
  String? _submitError;
  bool _busy = false;

  @override
  void dispose() {
    _addr.dispose();
    _alias.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final err = validateAddress(_chain, _addr.text);
    setState(() {
      _error = err;
      _submitError = null;
    });
    if (err != null) return;
    setState(() => _busy = true);
    try {
      await ref
          .read(apiProvider)
          .addWatch(_chain, _addr.text.trim(), alias: _alias.text.trim());
      ref.invalidate(watchlistProvider);
      if (mounted) Navigator.of(context).pop();
    } on ApiException catch (e) {
      if (mounted) setState(() => _submitError = e.message);
    } catch (_) {
      if (mounted) setState(() => _submitError = 'Could not add this address.');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('Add a wallet to watch',
                style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
            const SizedBox(height: 12),
            Wrap(spacing: 8, children: [
              for (final c in Chain.supported)
                ChoiceChip(
                  label: Text(c.label),
                  selected: _chain == c,
                  onSelected: (_) => setState(() {
                    _chain = c;
                    if (_error != null) {
                      _error = validateAddress(_chain, _addr.text);
                    }
                  }),
                ),
            ]),
            const SizedBox(height: 12),
            TextField(
              controller: _addr,
              autocorrect: false,
              enableSuggestions: false,
              decoration: InputDecoration(
                labelText: _chain.isEvm
                    ? '${_chain.label} address (0x...)'
                    : 'Solana address',
                errorText: _error,
                errorMaxLines: 3,
              ),
              onChanged: (_) {
                if (_error != null) {
                  setState(() => _error = validateAddress(_chain, _addr.text));
                }
              },
            ),
            const SizedBox(height: 12),
            TextField(
                controller: _alias,
                decoration:
                    const InputDecoration(labelText: 'Nickname (optional)')),
            if (_submitError != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Semantics(
                    liveRegion: true,
                    child: Text(_submitError!,
                        style: const TextStyle(color: TraceColors.negative))),
              ),
            const SizedBox(height: 16),
            SizedBox(
              width: double.infinity,
              child: FilledButton(
                  onPressed: _busy ? null : _submit,
                  child: Text(_busy ? 'Adding...' : 'Add to watchlist')),
            ),
            const SizedBox(height: 8),
            const Muted(
                'Public addresses only. Never paste a seed phrase or private key here.',
                size: 12),
          ]),
    );
  }
}
