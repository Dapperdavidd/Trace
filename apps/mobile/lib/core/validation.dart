import '../api/models.dart' show Chain;

final RegExp _evmPattern = RegExp(r'^0x[0-9a-fA-F]{40}$');
const String _base58Alphabet =
    '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

/// EVM address: 0x followed by exactly 40 hex characters.
bool isValidEvmAddress(String input) => _evmPattern.hasMatch(input.trim());

/// Solana address: base58 (no 0, O, I, l), 32 to 44 characters.
bool isValidSolanaAddress(String input) {
  final s = input.trim();
  if (s.length < 32 || s.length > 44) return false;
  for (final unit in s.codeUnits) {
    if (!_base58Alphabet.contains(String.fromCharCode(unit))) return false;
  }
  return true;
}

bool isValidAddressForChain(Chain chain, String input) {
  if (chain == Chain.solana) return isValidSolanaAddress(input);
  if (chain.isEvm) return isValidEvmAddress(input);
  return false;
}

/// Accepts a positive decimal amount no larger than [maximumWhole] without
/// converting money to a floating-point number.
bool isPositiveDecimalAtMost(String input, BigInt maximumWhole) {
  final match = RegExp(r'^\+?(\d+)(?:\.(\d+))?$').firstMatch(input.trim());
  if (match == null) return false;
  final whole = match.group(1) ?? '0';
  final fraction = match.group(2) ?? '';
  final coefficient = BigInt.parse('$whole$fraction');
  final scale = BigInt.from(10).pow(fraction.length);
  return coefficient > BigInt.zero && coefficient <= maximumWhole * scale;
}

/// Returns a plain-language error, or null when the address is acceptable.
String? validateAddress(Chain chain, String input) {
  final s = input.trim();
  if (chain == Chain.unknown) return 'Choose a chain first.';
  if (s.isEmpty) return 'Paste a wallet address.';
  if (chain == Chain.solana) {
    if (isValidSolanaAddress(s)) return null;
    return 'A Solana address is 32 to 44 letters and numbers. It never contains 0, O, I or l.';
  }
  if (isValidEvmAddress(s)) return null;
  return 'An ${chain.label} address starts with 0x followed by 40 hex characters (0-9, a-f).';
}
