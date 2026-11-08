import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../core/tokens.dart';
import '../state/session.dart';
import '../wallet/wallet_state.dart';
import '../widgets/common.dart';
import '../wallet/near_core.dart';

class YouScreen extends ConsumerWidget {
  const YouScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final wallet = ref.watch(walletProvider);
    return TraceScaffold(
      title: 'You',
      subtitle: 'Your account, wallet and settings.',
      actions: [
        IconButton(
          tooltip: 'Preferences',
          icon: const Icon(Icons.settings_outlined),
          onPressed: () => context.push('/more'),
        ),
      ],
      body: ListView(
        padding: const EdgeInsets.all(TraceSpacing.gutter),
        children: [
          TraceCard(
              child: Row(children: [
            const CircleAvatar(radius: 28, child: Icon(Icons.person_outline)),
            const SizedBox(width: 14),
            Expanded(
                child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                  Text(session.name ?? 'Dapper',
                      style: const TextStyle(
                          fontSize: 20, fontWeight: FontWeight.w600)),
                  Muted(session.email ?? 'Account profile', size: 13)
                ])),
            const Icon(Icons.chevron_right),
          ])),
          _walletCard(context, ref, wallet),
          TraceCard(
              child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceAround,
                  children: const [
                _Metric('3', 'Coins watched'),
                _Metric('2', 'Wallets followed'),
                _Metric('5', 'Shadow tests'),
              ])),
          _item(context, Icons.layers_outlined, 'Shadow settings',
              'Limits, mandates, and permissions', '/near'),
          _item(context, Icons.notifications_none, 'Alerts',
              'Price, wallet and risk notifications', '/alerts'),
          _item(context, Icons.shield_outlined, 'Security',
              'Passkey, devices and account control', '/account-sessions'),
          _item(context, Icons.extension_outlined, 'Connected agents',
              'Read and propose permissions only', '/near'),
          _item(context, Icons.tune, 'Preferences',
              'Currency, language and display', '/more'),
          const Disclaimer(),
        ],
      ),
    );
  }

  Widget _walletCard(BuildContext context, WidgetRef ref, WalletState state) {
    final current = state.current;
    return TraceCard(
      onTap: () async {
        if (current == null) {
          await _createWallet(context, ref);
        } else if (context.mounted) {
          context.push('/near');
        }
      },
      child: Row(children: [
        const Icon(Icons.account_balance_wallet_outlined,
            size: 32, color: TraceColors.emerald),
        const SizedBox(width: 14),
        Expanded(
            child:
                Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(current?.accountId ?? 'Create your NEAR wallet',
              style:
                  const TextStyle(fontSize: 17, fontWeight: FontWeight.w500)),
          Muted(
              current == null
                  ? 'Choose a unique testnet name to get started.'
                  : 'NEAR testnet wallet',
              size: 13)
        ])),
        const Icon(Icons.chevron_right),
      ]),
    );
  }

  Future<void> _createWallet(BuildContext context, WidgetRef ref) async {
    final controller = TextEditingController();
    final name = await showDialog<String>(
        context: context,
        builder: (ctx) => AlertDialog(
              title: const Text('Choose your NEAR name'),
              content: TextField(
                  controller: controller,
                  autofocus: true,
                  autocorrect: false,
                  decoration: const InputDecoration(hintText: 'your-name')),
              actions: [
                TextButton(
                    onPressed: () => Navigator.pop(ctx),
                    child: const Text('Cancel')),
                FilledButton(
                    onPressed: () => Navigator.pop(ctx, controller.text.trim()),
                    child: const Text('Create wallet'))
              ],
            ));
    if (name == null || name.isEmpty || !context.mounted) return;
    try {
      final w =
          await ref.read(walletProvider.notifier).create(NearNet.testnet, name);
      if (context.mounted)
        ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('Created ${w.accountId} on NEAR testnet.')));
    } catch (e) {
      if (context.mounted)
        ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('Could not create the NEAR account: $e')));
    }
  }

  Widget _item(BuildContext context, IconData icon, String title, String sub,
          String route) =>
      TraceCard(
        onTap: () => context.push(route),
        child: Row(children: [
          Icon(icon, color: TraceColors.text),
          const SizedBox(width: 14),
          Expanded(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                Text(title,
                    style: const TextStyle(
                        fontSize: 16, fontWeight: FontWeight.w500)),
                Muted(sub, size: 12)
              ])),
          const Icon(Icons.chevron_right)
        ]),
      );
}

class _Metric extends StatelessWidget {
  const _Metric(this.value, this.label);
  final String value, label;
  @override
  Widget build(BuildContext context) => Column(children: [
        Text(value,
            style: const TextStyle(fontSize: 25, fontWeight: FontWeight.w600)),
        Muted(label, size: 11)
      ]);
}
