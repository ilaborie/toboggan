# Improvement Ideas

## New Components

### IFrame web-component

Allow display a remote site.
See `~/Documents/Workspaces/ilaborie/slides-jug-summer-camp-24/src/components/window.rs`

```css
:root {
  --window-header-bg: ButtonFace;
  --window-header-fg: ButtonText;
  --window-body-bg: Canvas;

  --window-border-size: 0.125em;
  --window-radius: 0.25em;
  /* --window-shadow: 0 0 4px rgba(0, 0, 0, 0.8); */
  --window-shadow: var(--pico-box-shadow);
  --window-font: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    Roboto, Oxygen, Ubuntu, Cantarell, "Open Sans", "Helvetica Neue", sans-serif;

  --window-tool-size: .75em;
  --window-tool-close: #fd5f57;
  --window-tool-minify: #febc2f;
  --window-tool-expand: #28c840;
  --window-tool-fg: rgba(0, 0, 0, 0.8);

}


.window {
  /* outline: thin solid red; */
  display: flex;
  flex-direction: column;
  box-shadow: var(--window-shadow);
  border-radius: var(--window-radius);

  header,
  footer {
    background: var(--window-header-bg);
    color: var(--window-header-fg);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-family: var(--window-font);
    flex: 0 0 auto;
  }

  header {
    display: grid;
    grid-template-columns: 2em 1fr 2em;

    border-top-left-radius: var(--window-radius);
    border-top-right-radius: var(--window-radius);
    padding: 0.25em 0.5em;

    .left-tools {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: calc(var(--window-tool-size) / 2);

      :is(.close, .minify, .expand) {
        border: 0.5px solid var(--window-header-bg);
        border-radius: 50%;
        --size: var(--window-tool-size);
        max-width: var(--size);
        max-height: var(--size);
        min-width: var(--size);
        min-height: var(--size);
        cursor: pointer;
      }

      .close {
        background: var(--window-tool-close);
      }

      .minify {
        background: var(--window-tool-minify);
      }

      .expand {
        background: var(--window-tool-expand);
      }
    }

    .title {
      text-align: center;
      font-weight: bold;
      font-family: var(--pico-font-family-monospace);
    }

  }

  .body {
    flex-grow: 1;
    border-left: var(--window-border-size) solid var(--window-header-bg);
    border-right: var(--window-border-size) solid var(--window-header-bg);
    background: var(--window-body-bg);
    display: flex;
    flex-direction: column;
  }

  footer {
    border-bottom-left-radius: var(--window-radius);
    border-bottom-right-radius: var(--window-radius);
    min-height: var(--window-border-size);
  }
}

.browser {
  /* outline: thin solid green; */
  flex: 1 1 auto;
  overflow: auto;
}

```

### Picker web-component

Dialog + list + search box ()

### Term web-component

See `Documents/Workspaces/ilaborie/slides-jug-summer-camp-24/` term

https://github.com/junkdog/beamterm
https://junkdog.github.io/beamterm/

## Other

### UI
https://webdev.bryanhogan.com/start/ways-to-build/
https://shoelace.style/

https://ui.shadcn.com/docs/installation

### Logger

Create a logger wrapper


