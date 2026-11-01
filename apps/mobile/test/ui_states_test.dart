import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:trace_mobile/api/api_error.dart';
import 'package:trace_mobile/api/models.dart';
import 'package:trace_mobile/widgets/common.dart';

Widget _app(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  testWidgets('every data status has an explicit badge', (tester) async {
    const labels = <DataStatus, String>{
      DataStatus.live: 'LIVE',
      DataStatus.stale: 'STALE',
      DataStatus.partial: 'PARTIAL',
      DataStatus.unavailable: 'UNAVAILABLE',
      DataStatus.demo: 'DEMO DATA',
    };
    for (final entry in labels.entries) {
      await tester.pumpWidget(_app(EnvelopeBar(Envelope(status: entry.key))));
      expect(find.text(entry.value), findsOneWidget);
    }
  });

  testWidgets('empty state explains the absence of data', (tester) async {
    await tester.pumpWidget(_app(const EmptyState(
      title: 'No activity yet',
      message: 'New supported activity will appear here.',
    )));
    expect(find.text('No activity yet'), findsOneWidget);
    expect(
        find.text('New supported activity will appear here.'), findsOneWidget);
  });

  testWidgets('error state includes API code and request id', (tester) async {
    const error = ApiException(
      kind: ApiFailureKind.http,
      code: 'coverage_unavailable',
      message: 'Coverage could not be loaded.',
      status: 503,
      requestId: 'req-test-123',
    );
    await tester.pumpWidget(_app(ErrorState(error: error, onRetry: () {})));
    expect(find.text('Code: coverage_unavailable'), findsOneWidget);
    expect(find.text('Request id: req-test-123'), findsOneWidget);
    expect(find.text('Try again'), findsOneWidget);
  });
}
