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
import '../widgets/tiles.dart';

/// Bottom navigation "Shadow" tab: simulations and wallets to shadow.
class ShadowHomeScreen extends ConsumerWidget {
  const ShadowHomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final ids = ref.watch(shadowIdsProvider);
    final wallets = ref.watch(discoveryWalletsProvider('top'));
    return TraceScaffold(
      title: 'Shadow Trades',
      subtitle: 'See what would have happened if you followed a trader',
      body: ListView(
        padding: const EdgeInsets.all(TraceSpacing.gutter),
        children: [
          const TraceCard(
            child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Icon(Icons.science_outlined, color: TraceColors.emerald),
              SizedBox(width: 12),
              Expanded(child: Text(Copy.paperOnly)),
            ]),
          ),
          SizedBox(
            width: double.infinity,
            child: FilledButton(
              onPressed: () => context.push('/shadow/setup'),
              child: const Text('Start a paper simulation'),
            ),
          ),
          const SectionTitle('Your simulations'),
          if (ids.isEmpty)
            const Muted('None yet. Pick a wallet below or start with any address.'),
          for (final id in ids) _SimTile(id),
          const SectionTitle('Wallets you can shadow'),
          wallets.when(
            loading: () => const Center(child: Padding(padding: EdgeInsets.all(24), child: CircularProgressIndicator())),
            error: (e, _) => ErrorState(error: e, onRetry: () => ref.invalidate(discoveryWalletsProvider('top'))),
            data: (r) => Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              EnvelopeBar(r.env),
              for (final w in r.items)
                WalletTile(
                  w,
                  onTap: () => context.push('/shadow/setup?chain=${w.wallet.chain.wire}&address=${w.wallet.address}'),
                ),
            ]),
          ),
          const ComingLaterPanel(
            title: 'Auto Copy',
            explanation: 'Real copy trading is not available. Shadow simulations never trade real money.',
          ),
          const Disclaimer(),
        ],
      ),
    );
  }
}

class _SimTile extends ConsumerWidget {
  const _SimTile(this.id);
  final String id;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(shadowResultsProvider(id));
    final text = v.when(
      loading: () => 'Loading...',
      error: (_, __) => 'Unavailable',
      data: (r) => signedPercentString(r.returnPct),
    );
    return TraceCard(
      onTap: () => context.push('/shadow/results/$id'),
      semanticsLabel: 'Paper simulation $id, simulated result $text',
      child: Row(children: [
        const Icon(Icons.show_chart, color: TraceColors.emerald),
        const SizedBox(width: 12),
        Expanded(child: Text('Simulation $id', overflow: TextOverflow.ellipsis)),
        Text(text, style: const TextStyle(fontWeight: FontWeight.w800)),
      ]),
    );
  }
}

class ShadowResultsScreen extends ConsumerWidget {
  const ShadowResultsScreen({super.key, required this.id});
  final String id;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final v = ref.watch(shadowResultsProvider(id));
    return TraceScaffold(
      title: 'Shadow Trades',
      subtitle: 'Paper simulation results',
      body: AsyncBody<ShadowResults>(
        value: v,
        onRetry: () => ref.invalidate(shadowResultsProvider(id)),
        builder: (r) {
          final ret = parseDecimal(r.returnPct);
          final baselineReturn = r.baseline['return_pct'];
          return ListView(padding: const EdgeInsets.all(TraceSpacing.gutter), children: [
            EnvelopeBar(r.env),
            TraceCard(
              semanticsLabel: 'Your simulated result: ${signedPercentString(r.returnPct)}, ending value ${compactUsdString(r.endingUsd)}, starting with ${compactUsdString(r.startingUsd)}. Simulation, not real money.',
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                const Pill('Simulated, not real money', color: TraceColors.info),
                const SizedBox(height: 8),
                const Text('Your simulated result', style: TextStyle(color: TraceColors.textMuted)),
                Wrap(crossAxisAlignment: WrapCrossAlignment.center, spacing: 12, children: [
                  Text(signedPercentString(r.returnPct),
                      style: TextStyle(fontSize: 36, fontWeight: FontWeight.w800, color: changeColor(ret))),
                  Text(compactUsdString(r.endingUsd), style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700)),
                ]),
                Muted('Starting with ${compactUsdString(r.startingUsd)}'),
              ]),
            ),
            StatGrid([
              StatTile(label: 'Simulated trades', value: compactCount(r.trades)),
              StatTile(label: 'Win rate', value: plainPercentString(r.winRatePct, decimals: 0), note: 'Past simulated trades'),
              StatTile(label: 'Skipped trades', value: compactCount(r.skippedUnexecutable), note: 'Could not be filled at the delayed price'),
              StatTile(label: 'Fees and slippage', value: compactUsdString(r.feesAndSlippageUsd), note: 'Already included'),
            ]),
            const SectionTitle('Compared with a baseline'),
            TraceCard(
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text(r.baseline['label'] ?? 'Baseline'),
                Text(baselineReturn == null ? 'Not available' : signedPercentString(baselineReturn),
                    style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700)),
                const Muted('Shown so you can see whether following the wallet beat a simple alternative.', size: 12),
              ]),
            ),
            const SectionTitle('Simulation assumptions'),
            TraceCard(
              child: r.assumptions.isEmpty
                  ? const Muted('No assumptions reported.')
                  : Column(children: [
                      for (final e in r.assumptions.entries)
                        Padding(
                          padding: const EdgeInsets.symmetric(vertical: 2),
                          child: Row(children: [
                            Expanded(child: Muted(e.key.replaceAll('_', ' '))),
                            Text(e.value),
                          ]),
                        ),
                    ]),
            ),
            const Muted(Copy.paperOnly, size: 12),
            const Disclaimer(text: 'Simulated results use prices available after your chosen delay. They are not guaranteed and do not predict future results.'),
            Padding(
              padding: const EdgeInsets.only(bottom: 16),
              child: OutlinedButton(
                onPressed: () => context.push('/shadow/setup'),
                child: const Text('Try with a different amount'),
              ),
            ),
          ]);
        },
      ),
    );
  }
}

class ShadowSetupScreen extends ConsumerStatefulWidget {
  const ShadowSetupScreen({super.key, this.chain, this.address});
  final Chain? chain;
  final String? address;

  @override
  ConsumerState<ShadowSetupScreen> createState() => _ShadowSetupScreenState();
}

class _ShadowSetupScreenState extends ConsumerState<ShadowSetupScreen> {
  late Chain _chain = widget.chain ?? Chain.solana;
  late final TextEditingController _addr = TextEditingController(text: widget.address ?? '');
  final TextEditingController _custom = TextEditingController();
  String _budget = '100';
  bool _customBudget = false;
  String _size = 'fixed_usd';
  int _delay = 30;
  int _slippage = 100;
  String _exit = 'mirror_sells';
  String? _addrError;
  String? _budgetError;
  String? _submitError;
  bool _busy = false;

  @override
  void dispose() {
    _addr.dispose();
    _custom.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final addrErr = validateAddress(_chain, _addr.text);
    String? budgetErr;
    var budget = _budget;
    if (_customBudget) {
      final v = double.tryParse(_custom.text.trim());
      if (v == null || v <= 0 || v > 1000000) {
        budgetErr = 'Enter an amount above 0 and up to 1,000,000.';
      } else {
        budget = v.toString();
      }
    }
    setState(() {
      _addrError = addrErr;
      _budgetError = budgetErr;
      _submitError = null;
    });
    if (addrErr != null || budgetErr != null) return;
    setState(() => _busy = true);
    try {
      final id = await ref.read(apiProvider).createShadow(ShadowRequest(
            chain: _chain,
            address: _addr.text.trim(),
            budgetUsd: budget,
            sizeRule: _size,
            delaySeconds: _delay,
            slippageBps: _slippage,
            exitRule: _exit,
          ));
      ref.read(shadowIdsProvider.notifier).add(id);
      if (mounted) context.pushReplacement('/shadow/results/$id');
    } on ApiException catch (e) {
      if (mounted) setState(() => _submitError = e.message);
    } catch (_) {
      if (mounted) setState(() => _submitError = 'Could not start the simulation.');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Widget _choices<T>(String label, String help, List<(T, String)> options, T value, void Function(T) onPick) {
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      const SizedBox(height: 12),
      Text(label, style: const TextStyle(fontWeight: FontWeight.w700)),
      Muted(help, size: 12),
      const SizedBox(height: 4),
      Wrap(spacing: 8, children: [
        for (final o in options)
          ChoiceChip(label: Text(o.$2), selected: value == o.$1, onSelected: (_) => setState(() => onPick(o.$1))),
      ]),
    ]);
  }

  @override
  Widget build(BuildContext context) {
    final fixedWallet = widget.address != null && widget.address!.isNotEmpty;
    return TraceScaffold(
      title: 'Shadow setup',
      subtitle: 'Paper simulation. No real money.',
      body: ListView(padding: const EdgeInsets.all(TraceSpacing.gutter), children: [
        const TraceCard(
          child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Icon(Icons.science_outlined, color: TraceColors.emerald),
            SizedBox(width: 12),
            Expanded(child: Text('${Copy.paperOnly} TRACE replays what the wallet did using prices you could have got after a delay.')),
          ]),
        ),
        const SectionTitle('Wallet to shadow'),
        if (fixedWallet)
          TraceCard(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(shortAddress(widget.address!, head: 6, tail: 6), style: const TextStyle(fontWeight: FontWeight.w700)),
              Muted(_chain.label),
            ]),
          )
        else ...[
          Wrap(spacing: 8, children: [
            for (final c in Chain.supported)
              ChoiceChip(label: Text(c.label), selected: _chain == c, onSelected: (_) => setState(() => _chain = c)),
          ]),
          const SizedBox(height: 8),
          TextField(
            controller: _addr,
            decoration: InputDecoration(labelText: 'Wallet address', errorText: _addrError, errorMaxLines: 3),
            autocorrect: false,
            enableSuggestions: false,
          ),
        ],
        const SectionTitle('Simulation assumptions'),
        const Muted('These replace live risk controls. They change how the simulation is calculated, not what a real wallet does.', size: 12),
        _choices<String>('Starting budget (paper)', 'Pretend money used by the simulation.', const [
          ('50', '\$50'),
          ('100', '\$100'),
          ('250', '\$250'),
          ('custom', 'Custom'),
        ], _customBudget ? 'custom' : _budget, (v) {
          if (v == 'custom') {
            _customBudget = true;
          } else {
            _customBudget = false;
            _budget = v;
          }
        }),
        if (_customBudget)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: TextField(
              controller: _custom,
              keyboardType: const TextInputType.numberWithOptions(decimal: true),
              decoration: InputDecoration(labelText: 'Paper budget in USD', errorText: _budgetError),
            ),
          ),
        _choices<String>('Trade size', 'How big each paper trade is.', const [
          ('fixed_usd', 'Same amount each time'),
          ('fixed_pct', 'A fixed 5% of the budget'),
        ], _size, (v) => _size = v),
        _choices<int>('Entry delay', 'Followers always get in after the wallet. A longer delay is more realistic.', const [
          (0, 'None'),
          (30, '30 seconds'),
          (60, '1 minute'),
          (300, '5 minutes'),
        ], _delay, (v) => _delay = v),
        _choices<int>('Slippage allowance', 'Extra cost from price moving while you trade.', const [
          (50, '0.5%'),
          (100, '1%'),
          (200, '2%'),
        ], _slippage, (v) => _slippage = v),
        _choices<String>('Exit rule', 'When the paper position is sold.', const [
          ('mirror_sells', 'When the wallet sells'),
          ('time_24h', 'After 24 hours'),
          ('time_exit_7d', 'After 7 days'),
        ], _exit, (v) => _exit = v),
        const SizedBox(height: 16),
        if (_submitError != null)
          Semantics(liveRegion: true, child: Text(_submitError!, style: const TextStyle(color: TraceColors.negative))),
        const SizedBox(height: 8),
        FilledButton(
          onPressed: _busy ? null : _submit,
          child: Text(_busy ? 'Starting...' : 'Start paper simulation'),
        ),
        const DisabledAction(label: 'Auto Copy with real money'),
        const Disclaimer(),
      ]),
    );
  }
}
