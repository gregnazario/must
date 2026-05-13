export function renderApp(root: HTMLElement): void {
    const heading = document.createElement("h1");
    heading.textContent = "Platform Web";
    root.appendChild(heading);
}

export function version(): string {
    return "2.0.0";
}
