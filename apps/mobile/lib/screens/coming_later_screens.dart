import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../core/copy.dart';
import '../core/tokens.dart';
import '../widgets/common.dart';

const EdgeInsets _pad = EdgeInsets.all(TraceSpacing.gutter);

/// Wallet: spendable wallets are out of scope for the MVP. No balances are shown.
class WalletScreen extends StatelessWidget {
  const WalletScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return TraceScaffold(
      title: 'Wallet',
      subtitle: Copy.comingLater,
      body: ListView(padding: _pad, children: [
        const ComingLaterPanel(
          title: 'Your TRACE wallet',
          explanation:
              'A wallet that holds and sends your crypto is planned for a later version. Today TRACE never holds funds, so there is no balance to show.',
          icon: Icons.account_balance_wallet_outlined,
          children: [
            DisabledAction(label: 'Create wallet', icon: Icons.add_circle_outline),
            DisabledAction(label: 'Send', icon: Icons.north_east),
            DisabledAction(label: 'Receive', icon: Icons.south_west),
            DisabledAction(label: 'Buy', icon: Icons.add_card),
            DisabledAction(label: 'Swap', icon: Icons.swap_horiz),
          ],
        ),
        const SectionTitle('What you can do today'),
        TraceCard(
          onTap: () => context.go('/watchlist'),
          semanticsLabel: 'Watch any public wallet address',
          child: const Row(children: [
            Icon(Icons.visibility_outlined, color: TraceColors.emerald),
            SizedBox(width: 12),
            Expanded(child: Text('Watch any public wallet address. No keys needed.')),
            Icon(Icons.chevron_right),
          ]),
        ),
        const Disclaimer(text: Copy.watchOnly),
      ]),
    );
  }
}

class TradeScreen extends StatelessWidget {
  const TradeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return TraceScaffold(
      title: 'Trade & Swap',
      subtitle: Copy.comingLater,
      body: ListView(padding: _pad, children: [
        const ComingLaterPanel(
          title: 'Buy, sell and swap in the app',
          explanation:
              'Trading with real money is not available. TRACE does not sign or send transactions. Nothing here can move funds.',
          icon: Icons.swap_horiz,
          children: [
            SizedBox(height: 8),
            Wrap(spacing: 8, children: [
              ChoiceChip(label: Text('Swap'), selected: false, onSelected: null),
              ChoiceChip(label: Text('Buy'), selected: false, onSelected: null),
              ChoiceChip(label: Text('Sell'), selected: false, onSelected: null),
            ]),
            DisabledAction(label: 'Swap'),
          ],
        ),
        TraceCard(
          onTap: () => context.go('/shadow'),
          semanticsLabel: 'Practice with a paper simulation instead',
          child: const Row(children: [
            Icon(Icons.science_outlined, color: TraceColors.emerald),
            SizedBox(width: 12),
            Expanded(child: Text('Practice with a paper simulation instead. No real money.')),
            Icon(Icons.chevron_right),
          ]),
        ),
        const Disclaimer(),
      ]),
    );
  }
}

class AskScreen extends StatelessWidget {
  const AskScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return TraceScaffold(
      title: 'Ask TRACE',
      subtitle: Copy.comingLater,
      body: ListView(padding: _pad, children: [
        const ComingLaterPanel(
          title: 'Ask questions in plain words',
          explanation:
              'Natural-language research is planned for a later version. For now, use Top Movers, Token Research and the Daily Briefing.',
          icon: Icons.chat_bubble_outline,
        ),
        const Muted('Example questions (not available yet)'),
        const SizedBox(height: 8),
        for (final q in const [
          'Which coins are tracked wallets buying that have not pumped yet?',
          'Is this token risky?',
          'Why is this coin moving?',
        ])
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Semantics(
              enabled: false,
              label: 'Example question, not available: $q',
              child: ExcludeSemantics(
                child: Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    border: Border.all(color: TraceColors.border),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(q, style: const TextStyle(color: TraceColors.textMuted)),
                ),
              ),
            ),
          ),
        const SizedBox(height: 8),
        const TextField(
          enabled: false,
          decoration: InputDecoration(
            labelText: 'Ask anything about crypto (Coming later)',
            suffixIcon: Icon(Icons.send),
          ),
        ),
        const Disclaimer(),
      ]),
    );
  }
}
