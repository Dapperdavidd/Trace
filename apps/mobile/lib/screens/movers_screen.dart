import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/models.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';
import '../widgets/tiles.dart';

class MoversScreen extends ConsumerStatefulWidget {
  const MoversScreen({super.key});

  @override
  ConsumerState<MoversScreen> createState() => _MoversScreenState();
}

class _MoversScreenState extends ConsumerState<MoversScreen> {
  static const _tabs = [
    ('trending', 'Trending'),
    ('early_momentum', 'Early signals'),
    ('smart_money', 'Smart money'),
    ('new', 'New'),
  ];
  static const _windows = ['1h', '24h', '7d'];
  int _tab = 0;
  String _window = '24h';
  String _chain = '';

  @override
  Widget build(BuildContext context) {
    final q = (_tabs[_tab].$1, _window, _chain);
    final v = ref.watch(moversProvider(q));
    return TraceScaffold(
      title: 'Top Movers',
      subtitle: 'Coins moving now, with the reason and the risk',
      body: ListView(
        padding: const EdgeInsets.all(TraceSpacing.gutter),
        children: [
          Wrap(spacing: 8, children: [
            for (var i = 0; i < _tabs.length; i++)
              ChoiceChip(
                label: Text(_tabs[i].$2),
                selected: _tab == i,
                onSelected: (_) => setState(() => _tab = i),
              ),
          ]),
          const SizedBox(height: 8),
          Wrap(spacing: 8, crossAxisAlignment: WrapCrossAlignment.center, children: [
            const Muted('Window'),
            for (final w in _windows)
              ChoiceChip(label: Text(w), selected: _window == w, onSelected: (_) => setState(() => _window = w)),
          ]),
          const SizedBox(height: 8),
          Wrap(spacing: 8, crossAxisAlignment: WrapCrossAlignment.center, children: [
            const Muted('Chain'),
            ChoiceChip(label: const Text('All'), selected: _chain.isEmpty, onSelected: (_) => setState(() => _chain = '')),
            for (final c in Chain.supported)
              ChoiceChip(label: Text(c.label), selected: _chain == c.wire, onSelected: (_) => setState(() => _chain = c.wire)),
          ]),
          const SizedBox(height: 12),
          v.when(
            loading: () => const Padding(padding: EdgeInsets.all(32), child: Center(child: CircularProgressIndicator())),
            error: (e, _) => SizedBox(height: 320, child: ErrorState(error: e, onRetry: () => ref.invalidate(moversProvider(q)))),
            data: (r) => Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              EnvelopeBar(r.env),
              const Muted('Observed pool data. Thin liquidity can make a price jump look bigger than it is. Check the risk label before you act.', size: 12),
              const SizedBox(height: 12),
              if (r.items.isEmpty)
                const EmptyState(title: 'No tokens in this view', message: 'Nothing passed the activity and liquidity filters. Try another window or chain.'),
              for (final m in r.items) MoverTile(m),
              const Disclaimer(),
            ]),
          ),
        ],
      ),
    );
  }
}
