import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/models.dart';
import '../core/tokens.dart';
import '../state/providers.dart';
import '../widgets/common.dart';

class NearScreen extends ConsumerWidget {
  const NearScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final status = ref.watch(nearStatusProvider);
    final assets = ref.watch(nearAssetsProvider);
    final mandates = ref.watch(nearMandatesProvider);
    return TraceScaffold(
      title: 'NEAR Agents',
      subtitle: 'User-owned authority for Shadow agents',
      body: RefreshIndicator(
        onRefresh: () async {
          ref.invalidate(nearStatusProvider);
          ref.invalidate(nearAssetsProvider);
          ref.invalidate(nearMandatesProvider);
          await ref.read(nearStatusProvider.future);
        },
        child: ListView(
          padding: const EdgeInsets.all(TraceSpacing.gutter),
          children: [
            const _ArchitectureCard(),
            const SectionTitle('Control-plane status'),
            _Section<NearStatus>(
              value: status,
              onRetry: () => ref.invalidate(nearStatusProvider),
              builder: (value) => _StatusCard(value),
            ),
            const SectionTitle('Mandates'),
            _Section<NearMandatesResponse>(
              value: mandates,
              onRetry: () => ref.invalidate(nearMandatesProvider),
              builder: (value) => _Mandates(value),
            ),
            const SectionTitle('NEAR Intents assets'),
            _Section<NearAssetsResponse>(
              value: assets,
              onRetry: () => ref.invalidate(nearAssetsProvider),
              builder: (value) => _Assets(value),
            ),
            const Disclaimer(
              text:
                  'TRACE never holds your keys. Draft mandates have no authority. Only a transaction signed by the named NEAR owner and verified on-chain becomes active.',
            ),
          ],
        ),
      ),
    );
  }
}

class _ArchitectureCard extends StatelessWidget {
  const _ArchitectureCard();

  @override
  Widget build(BuildContext context) => const TraceCard(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Row(children: [
            Icon(Icons.hub_outlined, color: TraceColors.emerald),
            SizedBox(width: 10),
            Expanded(
              child: Text('NEAR is the core, not another market',
                  style: TextStyle(fontSize: 17, fontWeight: FontWeight.w800)),
            ),
          ]),
          SizedBox(height: 10),
          Muted('1. TRACE observes Ethereum and Solana activity.'),
          Muted('2. Shadow tests the strategy with paper money.'),
          Muted('3. A NEAR mandate defines exact agent limits.'),
          Muted('4. NEAR stores authority and evidence commitments.'),
        ]),
      );
}

class _StatusCard extends StatelessWidget {
  const _StatusCard(this.value);
  final NearStatus value;

  @override
  Widget build(BuildContext context) => TraceCard(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          EnvelopeBar(value.env),
          _StatusRow(
              'Network',
              value.network.toUpperCase(),
              value.isControlPlane
                  ? TraceColors.positive
                  : TraceColors.negative),
          _StatusRow(
              'Mandate contract',
              value.contract.deploymentVerified
                  ? 'VERIFIED'
                  : value.contract.configured
                      ? 'CONFIGURED, UNVERIFIED'
                      : 'UNAVAILABLE',
              value.contract.deploymentVerified
                  ? TraceColors.positive
                  : TraceColors.warning),
          _StatusRow(
              'Agent delegation',
              value.agent.delegationEnabled ? 'ENABLED' : 'OFF',
              value.agent.delegationEnabled
                  ? TraceColors.positive
                  : TraceColors.textMuted),
          if (value.contract.accountId != null)
            Muted('Contract: ${value.contract.accountId}', size: 11),
          if (value.agent.accountId != null)
            Muted('Agent: ${value.agent.accountId}', size: 11),
        ]),
      );
}

class _StatusRow extends StatelessWidget {
  const _StatusRow(this.label, this.value, this.color);
  final String label;
  final String value;
  final Color color;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.only(bottom: 10),
        child: Row(children: [
          Expanded(child: Text(label)),
          Pill(value, color: color),
        ]),
      );
}

class _Mandates extends StatelessWidget {
  const _Mandates(this.value);
  final NearMandatesResponse value;

  @override
  Widget build(BuildContext context) {
    if (value.items.isEmpty) {
      return const TraceCard(
        child: EmptyState(
          title: 'No NEAR mandates yet',
          message:
              'Create and sign a mandate in the TRACE web app. Native wallet signing is not available in this mobile build yet.',
        ),
      );
    }
    return Column(children: [
      EnvelopeBar(value.env),
      for (final item in value.items)
        TraceCard(
          child:
              Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Row(children: [
              Expanded(
                  child: Text(item.mode.replaceAll('_', ' '),
                      style: const TextStyle(fontWeight: FontWeight.w700))),
              Pill(item.status.toUpperCase(),
                  color: item.isAnchored
                      ? TraceColors.positive
                      : TraceColors.warning),
            ]),
            const SizedBox(height: 8),
            Muted('Owner: ${item.ownerAccount}'),
            Muted('Agent: ${item.agentAccount}'),
            Muted('Markets: ${item.allowedChains.join(', ')}'),
            Muted(
                'Per trade: \$${item.maxTradeUsd}  Daily: \$${item.maxDailyUsd}'),
            if (!item.isAnchored)
              const Padding(
                padding: EdgeInsets.only(top: 8),
                child: Muted('Draft only. This does not grant authority.'),
              ),
          ]),
        ),
    ]);
  }
}

class _Assets extends StatelessWidget {
  const _Assets(this.value);
  final NearAssetsResponse value;

  @override
  Widget build(BuildContext context) {
    if (value.items.isEmpty) {
      return const TraceCard(
        child: EmptyState(
          title: 'Assets unavailable',
          message: 'NEAR Intents did not return any supported assets.',
        ),
      );
    }
    final shown = value.items.take(12).toList();
    return TraceCard(
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        EnvelopeBar(value.env),
        for (final asset in shown)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Row(children: [
              TokenAvatar(asset.symbol, size: 34),
              const SizedBox(width: 10),
              Expanded(child: Text(asset.symbol)),
              Pill(asset.blockchain.toUpperCase(), color: TraceColors.info),
            ]),
          ),
        if (value.items.length > shown.length)
          Muted('${value.items.length - shown.length} more assets available.'),
      ]),
    );
  }
}

class _Section<T> extends StatelessWidget {
  const _Section({
    required this.value,
    required this.onRetry,
    required this.builder,
  });
  final AsyncValue<T> value;
  final VoidCallback onRetry;
  final Widget Function(T) builder;

  @override
  Widget build(BuildContext context) => value.when(
        data: builder,
        loading: () => const TraceCard(
          child: Center(child: CircularProgressIndicator()),
        ),
        error: (error, _) => TraceCard(
          child: ErrorState(error: error, onRetry: onRetry),
        ),
      );
}
