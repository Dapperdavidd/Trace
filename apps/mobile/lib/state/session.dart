import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:google_sign_in/google_sign_in.dart';

import '../api/api_error.dart';
import '../core/config.dart';
import 'providers.dart';

enum SessionPhase { loading, signedOut, explorer, signedIn }

class SessionState {
  const SessionState(this.phase,
      {this.name, this.email, this.error, this.busy = false});
  final SessionPhase phase;
  final String? name;
  final String? email;
  final String? error;
  final bool busy;

  SessionState copyWith({String? error, bool? busy, bool clearError = false}) =>
      SessionState(
        phase,
        name: name,
        email: email,
        error: clearError ? null : (error ?? this.error),
        busy: busy ?? this.busy,
      );
}

const _tokenKey = 'trace_access_token';
const _nameKey = 'trace_user_name';
const _emailKey = 'trace_user_email';

class SessionController extends Notifier<SessionState> {
  final FlutterSecureStorage _store = const FlutterSecureStorage();

  @override
  SessionState build() {
    Future<void>.microtask(_restore);
    return const SessionState(SessionPhase.loading);
  }

  Future<String?> _read(String k) async {
    try {
      return await _store.read(key: k);
    } catch (_) {
      return null;
    }
  }

  Future<void> _write(String k, String? v) async {
    try {
      if (v == null) {
        await _store.delete(key: k);
      } else {
        await _store.write(key: k, value: v);
      }
    } catch (_) {
      // Storage failure only means the session is not remembered.
    }
  }

  Future<void> _restore() async {
    final token = await _read(_tokenKey);
    if (token != null && token.isNotEmpty) {
      ref.read(apiClientProvider).accessToken = token;
      state = SessionState(SessionPhase.signedIn,
          name: await _read(_nameKey), email: await _read(_emailKey));
    } else {
      state = const SessionState(SessionPhase.signedOut);
    }
  }

  void explore() => state = const SessionState(SessionPhase.explorer);

  Future<void> signInWithGoogle() async {
    if (!AppConfig.googleConfigured) {
      state = state.copyWith(
          error: 'Google Sign-In is not configured in this build. '
              'You can still explore without signing in.');
      return;
    }
    state = state.copyWith(busy: true, clearError: true);
    try {
      final gs = GoogleSignIn(
        clientId:
            AppConfig.googleClientId.isEmpty ? null : AppConfig.googleClientId,
        serverClientId: AppConfig.googleServerClientId.isEmpty
            ? null
            : AppConfig.googleServerClientId,
        scopes: const ['email', 'profile'],
      );
      final account = await gs.signIn();
      if (account == null) {
        state = state.copyWith(busy: false);
        return;
      }
      final auth = await account.authentication;
      final idToken = auth.idToken;
      if (idToken == null) {
        state = state.copyWith(
            busy: false, error: 'Google did not return an ID token.');
        return;
      }
      final res = await ref.read(apiProvider).googleSignIn(idToken);
      ref.read(apiClientProvider).accessToken = res.accessToken;
      await _write(_tokenKey, res.accessToken);
      await _write(_nameKey, res.user.name);
      await _write(_emailKey, res.user.email);
      state = SessionState(SessionPhase.signedIn,
          name: res.user.name, email: res.user.email);
    } on ApiException catch (e) {
      state = state.copyWith(busy: false, error: e.message);
    } catch (_) {
      state = state.copyWith(
          busy: false, error: 'Sign-in failed. Please try again.');
    }
  }

  Future<void> signInForDevelopment() async {
    if (!AppConfig.devAuth) return;
    state = state.copyWith(busy: true, clearError: true);
    try {
      final res = await ref
          .read(apiProvider)
          .googleSignIn('dev:mobile-local@gifted.test');
      ref.read(apiClientProvider).accessToken = res.accessToken;
      await _write(_tokenKey, res.accessToken);
      await _write(_nameKey, res.user.name);
      await _write(_emailKey, res.user.email);
      state = SessionState(SessionPhase.signedIn,
          name: res.user.name, email: res.user.email);
    } on ApiException catch (e) {
      state = state.copyWith(
          busy: false,
          error:
              '${e.message}${e.requestId == null ? '' : ' Request id: ${e.requestId}'}');
    }
  }

  Future<void> signOut() async {
    try {
      await ref.read(apiProvider).logout();
    } catch (_) {}
    await _clear();
  }

  Future<void> deleteAccount() async {
    await ref.read(apiProvider).deleteAccount();
    await _clear();
  }

  Future<void> _clear() async {
    ref.read(apiClientProvider).accessToken = null;
    await _write(_tokenKey, null);
    await _write(_nameKey, null);
    await _write(_emailKey, null);
    state = const SessionState(SessionPhase.signedOut);
  }
}

final sessionProvider =
    NotifierProvider<SessionController, SessionState>(SessionController.new);
