import 'package:flutter_test/flutter_test.dart';
import 'package:pacer_app/main.dart';

void main() {
  testWidgets('PacerSQL app smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const PacerApp());
    expect(find.byType(PacerApp), findsOneWidget);
  });
}
