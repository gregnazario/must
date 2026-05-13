(function () {
  "use strict";

  var PLAYGROUNDS = {
    "getting-started": {
      examples: [
        {
          label: "must build",
          command: "must build",
          output: [
            { text: "must build", cls: "input" },
            { text: "  Building [build] ...", cls: "dim" },
            { text: "  Building [lint] ...", cls: "dim" },
            { text: "  Building [test] ...", cls: "dim" },
            { text: "3 built, 0 cached, 0 failed \u2014 1.2s", cls: "success" },
          ],
        },
        {
          label: "must test",
          command: "must test",
          output: [
            { text: "must test", cls: "input" },
            { text: "  Running [test] ...", cls: "dim" },
            { text: "  running 893 tests", cls: "info" },
            { text: "  test result: ok. 893 passed; 0 failed", cls: "success" },
            { text: "1 built, 0 cached, 0 failed \u2014 3.4s", cls: "success" },
          ],
        },
        {
          label: "must list",
          command: "must list",
          output: [
            { text: "must list", cls: "input" },
            { text: "RECIPE   TYPE    STATUS  DEPS", cls: "dim" },
            { text: "lint     shell   ok      ", cls: "" },
            { text: "build    shell   ok      lint", cls: "" },
            { text: "test     shell   ok      build", cls: "" },
            { text: "run      shell   ok      build", cls: "" },
          ],
        },
        {
          label: "must doctor",
          command: "must doctor",
          output: [
            { text: "must doctor", cls: "input" },
            { text: "  rustc   1.85.0  \u2713", cls: "success" },
            { text: "  go      1.23.4  \u2713", cls: "success" },
            { text: "  node    22.5.0  \u2713", cls: "success" },
            { text: "  python  3.13.1  \u2713", cls: "success" },
            { text: "  zig     0.13.0  \u2713", cls: "success" },
          ],
        },
        {
          label: "must explain build",
          command: "must explain build",
          output: [
            { text: "must explain build", cls: "input" },
            { text: "Recipe: build", cls: "info" },
            { text: "  Type:      shell", cls: "" },
            { text: "  Script:    cargo build", cls: "" },
            { text: "  Cache:     hash", cls: "" },
            { text: "  Inputs:    src/**/*.rs, Cargo.toml", cls: "" },
            { text: "  Outputs:   target/debug/myapp", cls: "" },
            { text: "  Deps:      lint", cls: "" },
          ],
        },
      ],
    },
    rust: {
      examples: [
        {
          label: "must build",
          command: "must build",
          output: [
            { text: "must build", cls: "input" },
            { text: "  Compiling myapp v0.1.0 ...", cls: "dim" },
            { text: "  Finished `dev` profile [unoptimized]", cls: "info" },
            { text: "1 built, 0 cached, 0 failed \u2014 2.1s", cls: "success" },
          ],
        },
        {
          label: "must build --release",
          command: "must build --profile release",
          output: [
            { text: "must build --profile release", cls: "input" },
            { text: "  Compiling myapp v0.1.0 ...", cls: "dim" },
            { text: "  Finished `release` profile [optimized]", cls: "info" },
            { text: "1 built, 0 cached, 0 failed \u2014 8.4s", cls: "success" },
          ],
        },
        {
          label: "must test",
          command: "must test",
          output: [
            { text: "must test", cls: "input" },
            { text: "  running 42 tests ...", cls: "dim" },
            { text: "  test result: ok. 42 passed; 0 failed", cls: "success" },
            { text: "1 built, 0 cached, 0 failed \u2014 1.8s", cls: "success" },
          ],
        },
      ],
    },
    go: {
      examples: [
        {
          label: "must build",
          command: "must build",
          output: [
            { text: "must build", cls: "input" },
            { text: "  Building ./cmd/myapp ...", cls: "dim" },
            { text: "1 built, 0 cached, 0 failed \u2014 0.9s", cls: "success" },
          ],
        },
        {
          label: "must test",
          command: "must test",
          output: [
            { text: "must test", cls: "input" },
            { text: "  ok  github.com/user/myapp  0.234s", cls: "success" },
            { text: "1 built, 0 cached, 0 failed \u2014 0.6s", cls: "success" },
          ],
        },
      ],
    },
    python: {
      examples: [
        {
          label: "must build",
          command: "must build",
          output: [
            { text: "must build", cls: "input" },
            { text: "  Building mypkg ...", cls: "dim" },
            { text: "  Successfully built mypkg", cls: "success" },
            { text: "1 built, 0 cached, 0 failed \u2014 3.2s", cls: "success" },
          ],
        },
        {
          label: "must test",
          command: "must test",
          output: [
            { text: "must test", cls: "input" },
            { text: "  ==== 12 passed in 0.45s ====", cls: "success" },
            { text: "1 built, 0 cached, 0 failed \u2014 1.1s", cls: "success" },
          ],
        },
      ],
    },
    typescript: {
      examples: [
        {
          label: "must build",
          command: "must build",
          output: [
            { text: "must build", cls: "input" },
            { text: "  Compiling myapp via tsup ...", cls: "dim" },
            { text: "  dist/index.js  12.4kb", cls: "info" },
            { text: "1 built, 0 cached, 0 failed \u2014 1.8s", cls: "success" },
          ],
        },
        {
          label: "npm run lint",
          command: "must run lint",
          output: [
            { text: "must run lint", cls: "input" },
            { text: "  Running eslint ...", cls: "dim" },
            { text: "  0 errors, 0 warnings", cls: "success" },
            { text: "1 built, 0 cached, 0 failed \u2014 0.9s", cls: "success" },
          ],
        },
      ],
    },
    docker: {
      examples: [
        {
          label: "must build",
          command: "must build",
          output: [
            { text: "must build", cls: "input" },
            { text: "  Building image myapp:latest ...", cls: "dim" },
            { text: "  DONE 3.2s", cls: "info" },
            { text: "1 built, 0 cached, 0 failed \u2014 4.1s", cls: "success" },
          ],
        },
      ],
    },
    bridge: {
      examples: [
        {
          label: "must build (auto)",
          command: "must build",
          output: [
            { text: "must build", cls: "input" },
            { text: "  [bridge] detected Makefile, delegating to make", cls: "dim" },
            { text: "  Building project ...", cls: "dim" },
            { text: "1 built, 0 cached, 0 failed \u2014 2.3s", cls: "success" },
          ],
        },
        {
          label: "must list",
          command: "must list",
          output: [
            { text: "must list", cls: "input" },
            { text: "  [bridge] detected package.json", cls: "dim" },
            { text: "RECIPE      TYPE    STATUS", cls: "dim" },
            { text: "build       bridge  ok", cls: "" },
            { text: "test        bridge  ok", cls: "" },
            { text: "lint        bridge  ok", cls: "" },
          ],
        },
      ],
    },
  };

  function escapeHtml(s) {
    var d = document.createElement("div");
    d.appendChild(document.createTextNode(s));
    return d.innerHTML;
  }

  function createTerminal(id, playground) {
    var container = document.getElementById(id);
    if (!container) return;

    var term = document.createElement("div");
    term.className = "must-terminal";

    var bar = document.createElement("div");
    bar.className = "must-terminal-bar";
    bar.innerHTML =
      '<span class="must-terminal-dot must-terminal-dot--red"></span>' +
      '<span class="must-terminal-dot must-terminal-dot--yellow"></span>' +
      '<span class="must-terminal-dot must-terminal-dot--green"></span>' +
      '<span class="must-terminal-title">must playground</span>';

    var body = document.createElement("div");
    body.className = "must-terminal-body";

    var inputRow = document.createElement("div");
    inputRow.className = "must-terminal-input-row";
    inputRow.innerHTML =
      '<span class="must-terminal-prompt">$\u00a0</span>' +
      '<input class="must-terminal-input" type="text" spellcheck="false" autocomplete="off" placeholder="Type a command (try: must build)">';

    term.appendChild(bar);
    term.appendChild(body);
    body.appendChild(inputRow);

    var input = inputRow.querySelector(".must-terminal-input");

    function addLine(text, cls) {
      var line = document.createElement("p");
      line.className = "must-terminal-line" + (cls ? " must-terminal-line--" + cls : "");
      if (cls === "input") {
        line.innerHTML =
          '<span class="must-terminal-prompt">$\u00a0</span>' +
          escapeHtml(text);
      } else {
        line.textContent = text;
      }
      body.insertBefore(line, inputRow);
      body.scrollTop = body.scrollHeight;
    }

    function runScenario(scenario) {
      var lines = body.querySelectorAll(".must-terminal-line");
      for (var i = 0; i < lines.length; i++) {
        lines[i].remove();
      }

      var idx = 0;
      function next() {
        if (idx >= scenario.output.length) return;
        var item = scenario.output[idx];
        addLine(item.text, item.cls);
        idx++;
        if (idx < scenario.output.length) {
          setTimeout(next, 60 + Math.random() * 80);
        }
      }
      next();
    }

    input.addEventListener("keydown", function (e) {
      if (e.key === "Enter") {
        e.preventDefault();
        var cmd = input.value.trim();
        if (!cmd) return;

        addLine(cmd, "input");
        input.value = "";

        for (var i = 0; i < playground.examples.length; i++) {
          if (playground.examples[i].command === cmd) {
            var scenario = playground.examples[i];
            var delay = 0;
            for (var j = 0; j < scenario.output.length; j++) {
              (function (item) {
                setTimeout(function () {
                  addLine(item.text, item.cls);
                }, delay);
                delay += 60 + Math.random() * 80;
              })(scenario.output[j]);
            }
            return;
          }
        }

        setTimeout(function () {
          addLine("error: unknown command '" + cmd + "'", "error");
          addLine("Try: must build, must test, must list, must doctor, must explain <recipe>", "dim");
        }, 100);
      }
    });

    container.appendChild(term);
  }

  function createPlaygroundWidget(id, playgroundKey) {
    var container = document.getElementById(id);
    if (!container) return;

    var playground = PLAYGROUNDS[playgroundKey];
    if (!playground) return;

    var wrapper = document.createElement("div");
    wrapper.className = "must-playground";

    var label = document.createElement("div");
    label.className = "must-playground-label";
    label.textContent = "Try it";
    wrapper.appendChild(label);

    if (playground.examples.length > 1) {
      var btns = document.createElement("div");
      btns.className = "must-playground-examples";
      playground.examples.forEach(function (ex, i) {
        var btn = document.createElement("button");
        btn.className = "must-playground-btn" + (i === 0 ? " must-playground-btn--active" : "");
        btn.textContent = ex.label;
        btn.addEventListener("click", function () {
          var all = btns.querySelectorAll(".must-playground-btn");
          for (var j = 0; j < all.length; j++) all[j].classList.remove("must-playground-btn--active");
          btn.classList.add("must-playground-btn--active");
          var term = wrapper.querySelector(".must-terminal");
          if (term) {
            var body = term.querySelector(".must-terminal-body");
            var lines = body.querySelectorAll(".must-terminal-line");
            for (var k = 0; k < lines.length; k++) lines[k].remove();
            var input = body.querySelector(".must-terminal-input");
            if (input) {
              input.value = "";
              input.focus();
            }
            runScenarioInTerminal(term, ex);
          }
        });
        btns.appendChild(btn);
      });
      wrapper.appendChild(btns);
    }

    var termId = id + "-term";
    var termDiv = document.createElement("div");
    termDiv.id = termId;
    wrapper.appendChild(termDiv);

    container.appendChild(wrapper);

    createTerminal(termId, playground);

    if (playground.examples.length > 0) {
      var term = document.getElementById(termId);
      if (term) {
        runScenarioInTerminal(term.querySelector(".must-terminal"), playground.examples[0]);
      }
    }
  }

  function runScenarioInTerminal(termEl, scenario) {
    if (!termEl) return;
    var body = termEl.querySelector(".must-terminal-body");
    var inputRow = body.querySelector(".must-terminal-input-row");
    var idx = 0;

    function next() {
      if (idx >= scenario.output.length) return;
      var item = scenario.output[idx];
      var line = document.createElement("p");
      line.className = "must-terminal-line" + (item.cls ? " must-terminal-line--" + item.cls : "");
      if (item.cls === "input") {
        line.innerHTML =
          '<span class="must-terminal-prompt">$\u00a0</span>' +
          escapeHtml(item.text);
      } else {
        line.textContent = item.text;
      }
      body.insertBefore(line, inputRow);
      body.scrollTop = body.scrollHeight;
      idx++;
      if (idx < scenario.output.length) {
        setTimeout(next, 80 + Math.random() * 60);
      }
    }
    next();
  }

  function init() {
    document.querySelectorAll("[data-must-playground]").forEach(function (el) {
      createPlaygroundWidget(el.id, el.getAttribute("data-must-playground"));
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
