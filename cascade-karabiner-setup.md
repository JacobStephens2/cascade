# Remap Play/Pause to Open Cascade

## Prerequisites

- macOS
- [Homebrew](https://brew.sh)

## 1. Fix Homebrew Permissions

```sh
sudo chown -R $(whoami) /opt/homebrew
```

## 2. Install Karabiner-Elements

```sh
brew install --cask karabiner-elements
```

## 3. Grant System Permissions

When you first open Karabiner-Elements, macOS will prompt you to allow permissions. Go to **System Settings > Privacy & Security** and enable:

- **Input Monitoring** — for `karabiner_grabber` and `karabiner_observer`
- **Accessibility** — for `karabiner_grabber`

You may need to restart Karabiner-Elements after granting permissions.

## 4. Configuration

The config file at `~/.config/karabiner/karabiner.json` has already been set up with a rule that remaps the play/pause media key to open the Cascade app.

To verify or edit the rule, open Karabiner-Elements and go to **Complex Modifications**. You should see:

> Play/Pause opens Cascade instead of Apple Music

## Troubleshooting

- **Play/pause still opens Apple Music:** Make sure both Input Monitoring and Accessibility permissions are granted, then restart Karabiner-Elements.
- **Cascade doesn't open:** Confirm the app is installed at `/Applications/Cascade.app`. If it's installed elsewhere, update the `shell_command` in `~/.config/karabiner/karabiner.json` with the correct path.
- **Key works only sometimes:** Some apps (e.g., Spotify) aggressively grab media keys. Quit those apps to let Karabiner intercept the key first.
