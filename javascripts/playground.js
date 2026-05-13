"use strict";
(function () {
    "use strict";
    const PLAYGROUNDS = {
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
        const d = document.createElement("div");
        d.appendChild(document.createTextNode(s));
        return d.innerHTML;
    }
    function addTerminalLine(body, inputRow, text, cls) {
        const line = document.createElement("p");
        line.className = "must-terminal-line" + (cls ? " must-terminal-line--" + cls : "");
        if (cls === "input") {
            line.innerHTML = '<span class="must-terminal-prompt">$\u00a0</span>' + escapeHtml(text);
        }
        else {
            line.textContent = text;
        }
        body.insertBefore(line, inputRow);
        body.scrollTop = body.scrollHeight;
    }
    function clearTerminalLines(body) {
        const lines = body.querySelectorAll(".must-terminal-line");
        lines.forEach((el) => el.remove());
    }
    function scheduleScenarioOutput(body, inputRow, scenario) {
        let delay = 0;
        for (const item of scenario.output) {
            setTimeout(() => addTerminalLine(body, inputRow, item.text, item.cls), delay);
            delay += 60 + Math.random() * 80;
        }
    }
    function createTerminal(container, playground) {
        const term = document.createElement("div");
        term.className = "must-terminal";
        const bar = document.createElement("div");
        bar.className = "must-terminal-bar";
        bar.innerHTML =
            '<span class="must-terminal-dot must-terminal-dot--red"></span>' +
                '<span class="must-terminal-dot must-terminal-dot--yellow"></span>' +
                '<span class="must-terminal-dot must-terminal-dot--green"></span>' +
                '<span class="must-terminal-title">must playground</span>';
        const body = document.createElement("div");
        body.className = "must-terminal-body";
        const inputRow = document.createElement("div");
        inputRow.className = "must-terminal-input-row";
        inputRow.innerHTML =
            '<span class="must-terminal-prompt">$\u00a0</span>' +
                '<input class="must-terminal-input" type="text" spellcheck="false" autocomplete="off" placeholder="Type a command (try: must build)">';
        term.appendChild(bar);
        term.appendChild(body);
        body.appendChild(inputRow);
        const input = inputRow.querySelector(".must-terminal-input");
        input.addEventListener("keydown", (e) => {
            if (e.key === "Enter") {
                e.preventDefault();
                const cmd = input.value.trim();
                if (!cmd)
                    return;
                addTerminalLine(body, inputRow, cmd, "input");
                input.value = "";
                const match = playground.examples.find((ex) => ex.command === cmd);
                if (match) {
                    scheduleScenarioOutput(body, inputRow, match);
                    return;
                }
                setTimeout(() => {
                    addTerminalLine(body, inputRow, "error: unknown command '" + cmd + "'", "error");
                    addTerminalLine(body, inputRow, "Try: must build, must test, must list, must doctor, must explain <recipe>", "dim");
                }, 100);
            }
        });
        container.appendChild(term);
    }
    function runScenarioInTerminal(termEl, scenario) {
        if (!termEl)
            return;
        const body = termEl.querySelector(".must-terminal-body");
        const inputRow = body.querySelector(".must-terminal-input-row");
        let idx = 0;
        function next() {
            if (idx >= scenario.output.length)
                return;
            const item = scenario.output[idx];
            addTerminalLine(body, inputRow, item.text, item.cls);
            idx++;
            if (idx < scenario.output.length) {
                setTimeout(next, 80 + Math.random() * 60);
            }
        }
        next();
    }
    function resetTerminalAndRun(wrapper, scenario) {
        const term = wrapper.querySelector(".must-terminal");
        if (!term)
            return;
        const body = term.querySelector(".must-terminal-body");
        clearTerminalLines(body);
        const input = body.querySelector(".must-terminal-input");
        if (input) {
            input.value = "";
            input.focus();
        }
        runScenarioInTerminal(term, scenario);
    }
    function createPlaygroundWidget(container, playgroundKey) {
        const playground = PLAYGROUNDS[playgroundKey];
        if (!playground)
            return;
        const wrapper = document.createElement("div");
        wrapper.className = "must-playground";
        const label = document.createElement("div");
        label.className = "must-playground-label";
        label.textContent = "Try it";
        wrapper.appendChild(label);
        if (playground.examples.length > 1) {
            const btns = document.createElement("div");
            btns.className = "must-playground-examples";
            playground.examples.forEach((ex, i) => {
                const btn = document.createElement("button");
                btn.className = "must-playground-btn" + (i === 0 ? " must-playground-btn--active" : "");
                btn.textContent = ex.label;
                btn.addEventListener("click", () => {
                    const all = btns.querySelectorAll(".must-playground-btn");
                    all.forEach((b) => b.classList.remove("must-playground-btn--active"));
                    btn.classList.add("must-playground-btn--active");
                    resetTerminalAndRun(wrapper, ex);
                });
                btns.appendChild(btn);
            });
            wrapper.appendChild(btns);
        }
        const termDiv = document.createElement("div");
        wrapper.appendChild(termDiv);
        container.appendChild(wrapper);
        createTerminal(termDiv, playground);
        if (playground.examples.length > 0) {
            const term = termDiv.querySelector(".must-terminal");
            if (term) {
                runScenarioInTerminal(term, playground.examples[0]);
            }
        }
    }
    function init() {
        document.querySelectorAll("[data-must-playground]").forEach((el) => {
            createPlaygroundWidget(el, el.getAttribute("data-must-playground"));
        });
    }
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", init);
    }
    else {
        init();
    }
})();
