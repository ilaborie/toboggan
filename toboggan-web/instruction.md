# Code review


## main.ts
- introduce an enum for button id like `first-btn`
- when using `getRequiredElement` use the more generic element type (Open/Close principle)
- create constantes for `Command`
- `AppConfig should use env. var. with default fallback. See <https://vite.dev/guide/env-and-mode>
- try to reduce the size main.ts file by extracting code in sub-modules

## dom.ts

We should have a proper Errors component instead of `showError`
This component should handle the display/hide status of the current error.
It should be in a proper error.ts file.
And also display error in the console.

##
