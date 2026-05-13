import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_app/main.dart';

void main() {
  testWidgets('renders hello text', (WidgetTester tester) async {
    await tester.pumpWidget(const FlutterApp());
    expect(find.text('Hello, world!'), findsOneWidget);
  });
}
