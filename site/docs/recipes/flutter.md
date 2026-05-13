# Flutter Recipes

must provides two Flutter recipe types: `flutter-build` and `flutter-test`.

## Quick start

Project layout:

```
myapp/
├── Mustfile.toml
├── pubspec.yaml
├── lib/
│   └── main.dart
└── test/
    └── widget_test.dart
```

`pubspec.yaml`:

```yaml
name: myapp
version: 1.0.0

environment:
  sdk: ">=3.0.0 <4.0.0"

dependencies:
  flutter:
    sdk: flutter
  cupertino_icons: ^1.0.6

dev_dependencies:
  flutter_test:
    sdk: flutter
```

`lib/main.dart`:

```dart
import 'package:flutter/material.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(title: const Text('My App')),
        body: const Center(
          child: Text('Hello from Flutter!'),
        ),
      ),
    );
  }
}
```

`test/widget_test.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:myapp/main.dart';

void main() {
  testWidgets('renders hello text', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('Hello from Flutter!'), findsOneWidget);
  });
}
```

`Mustfile.toml`:

```toml
[project]
name = "myapp"
version = "1.0.0"

[recipe.build]
type    = "flutter-build"
package = "."
inputs  = ["lib/**/*.dart", "pubspec.yaml"]
outputs = ["build/**/*"]

[recipe.test]
type    = "flutter-test"
package = "."
deps    = ["build"]
cache   = "none"
```

Build the app:

```
$ must build
[build] flutter build apk
Building without sound null safety
Running Gradle task: assembleRelease...
✓ Built build/app/outputs/flutter-apk/app-release.apk (12.3MB)
```

Run the tests:

```
$ must test
[test] flutter test
00:01 +1: renders hello text
All tests passed!
```

## `flutter-build` — Build a Flutter app

```toml
[recipe.build]
type    = "flutter-build"
package = "."
```

Runs: `flutter build <platform>` (platform is derived from target: `apk`, `ios`, `web`, `macos`, `windows`, `linux`)

Cache key includes: `flutter --version`, package, target, env vars.

## `flutter-test` — Run tests

```toml
[recipe.test]
type    = "flutter-test"
package = "."
deps    = ["build"]
```

Runs: `flutter test` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## Cross-platform builds

The build platform is determined by the target:

| Target | Platform |
|--------|----------|
| `android`, `android-arm`, `android-arm64` | `apk` |
| `ios` | `ios` |
| `web` | `web` |
| `macos` | `macos` |
| `windows` | `windows` |
| `linux` | `linux` |

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Project directory path (default `.`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Examples

```toml
[project]
name = "my-flutter-app"

[recipe.build]
type    = "flutter-build"
package = "."

[recipe.test]
type    = "flutter-test"
package = "."
deps    = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Dart Recipes](dart.md) — Dart build and test
