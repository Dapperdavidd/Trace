// ignore_for_file: avoid_print
// Manual end-to-end check of the wallet core against NEAR testnet. Not run in CI.
import 'package:trace_mobile/wallet/near_core.dart';

Future<void> main() async {
  final key = NearKey.random();
  final id = testnetName('dartcheck');
  await faucetCreate(id, key.publicKey);
  final rpc = NearRpc(NearNet.testnet);
  BigInt? b;
  for (var i = 0; i < 12 && b == null; i++) {
    b = await rpc.balance(id);
    if (b == null) await Future<void>.delayed(const Duration(milliseconds: 1200));
  }
  print('created $id balance ${yoctoToNear(b ?? BigInt.zero)}');
  final sweep = await requestTestnetTokens(id);
  print('sweep tx $sweep');
  await Future<void>.delayed(const Duration(seconds: 3));
  print('after sweep ${yoctoToNear((await rpc.balance(id))!)}');
  final send = await rpc.transfer(id, key, 'testnet', nearToYocto('1'));
  print('send tx $send');
  await Future<void>.delayed(const Duration(seconds: 3));
  print('after send ${yoctoToNear((await rpc.balance(id))!)}');
  final round = NearKey.parse(key.secretKey);
  print('key round trip ${round.publicKey == key.publicKey} implicit ${key.implicitAccount.length}');
}
