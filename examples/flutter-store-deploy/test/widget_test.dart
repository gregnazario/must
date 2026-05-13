import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:my_flutter_app/main.dart';

void main() {
  testWidgets('renders greeting', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('Hello, world!'), findsOneWidget);
  });
}
