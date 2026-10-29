import 'package:flutter_test/flutter_test.dart';
import 'package:trace_mobile/api/demo_api.dart';
import 'package:trace_mobile/api/models.dart';
import 'package:trace_mobile/core/validation.dart';

void main() {
  group('EVM addresses', () {
    const good = '0x52908400098527886E0F7030069857D2E4169EE7';
    test('accepts 0x plus 40 hex', () {
      expect(isValidEvmAddress(good), isTrue);
      expect(isValidEvmAddress(good.toLowerCase()), isTrue);
      expect(isValidEvmAddress('  $good  '), isTrue);
    });
    test('rejects bad length, prefix and characters', () {
      expect(isValidEvmAddress('0x123'), isFalse);
      expect(isValidEvmAddress(good.substring(2)), isFalse);
      expect(isValidEvmAddress('${good}0'), isFalse);
      expect(isValidEvmAddress('0x${'g' * 40}'), isFalse);
      expect(isValidEvmAddress('0X${'a' * 40}'), isFalse);
      expect(isValidEvmAddress(''), isFalse);
    });
  });

  group('Solana addresses', () {
    test('accepts base58 of 32 to 44 chars', () {
      expect(isValidSolanaAddress(DemoApi.demoWalletAddress), isTrue);
      expect(isValidSolanaAddress(DemoApi.demoThinWalletAddress), isTrue);
      expect(isValidSolanaAddress('1' * 32), isTrue);
      expect(isValidSolanaAddress('A' * 44), isTrue);
    });
    test('rejects wrong length', () {
      expect(isValidSolanaAddress('A' * 31), isFalse);
      expect(isValidSolanaAddress('A' * 45), isFalse);
    });
    test('rejects characters outside base58 (0, O, I, l)', () {
      for (final bad in ['0', 'O', 'I', 'l']) {
        expect(isValidSolanaAddress('${'A' * 40}$bad'), isFalse, reason: bad);
      }
      expect(isValidSolanaAddress('${'A' * 40}-_ '), isFalse);
    });
  });

  group('validateAddress', () {
    const evm = '0x52908400098527886E0F7030069857D2E4169EE7';
    test('is chain specific', () {
      expect(validateAddress(Chain.ethereum, evm), isNull);
      expect(validateAddress(Chain.base, evm), isNull);
      expect(validateAddress(Chain.bnb, evm), isNull);
      expect(validateAddress(Chain.solana, evm), isNotNull);
      expect(validateAddress(Chain.ethereum, DemoApi.demoWalletAddress), isNotNull);
      expect(validateAddress(Chain.solana, DemoApi.demoWalletAddress), isNull);
    });
    test('empty and unknown chain', () {
      expect(validateAddress(Chain.solana, '  '), 'Paste a wallet address.');
      expect(validateAddress(Chain.unknown, evm), 'Choose a chain first.');
    });
  });
}
