# New Commands Documentation

## Coinflip Command

### Overview
The `coinflip` command simulates a coin flip and announces the result with a fun message.

### Usage
```
/coinflip
-coinflip
```

No parameters needed - just run the command!

### Examples

**Input:**
```
/coinflip
```

**Animation Sequence:**
1. First shows: `🪙 **Alice** is flipping a coin...`
2. Animates through: `🔄 **Alice** is flipping a coin...`
3. Continues flipping back and forth for ~2 seconds
4. Finally reveals: `🪙 **Alice**! The coin landed on **Heads**!`

**Possible Final Results:**
```
🪙 **Alice**! The coin landed on **Heads**!
```
```
🔄 **Alice**! The coin landed on **Tails**!
```

### Features
- **Animated Flip:** Shows a fun 2-second animation of the coin flipping back and forth
- **Random Results:** Uses secure random number generation for fair coin flips
- **Visual Feedback:** Different emojis for heads (🪙) and tails (🔄)
- **Personal Touch:** Mentions the user who ran the command
- **Message Editing:** Uses Discord's message editing to create smooth animation
- **Both Command Types:** Works as both slash command (`/coinflip`) and prefix command (`-coinflip`)

---

## Profile Picture (PFP) Command

### Overview
The `pfp` command displays the profile picture of a mentioned user in a nice embed format.

### Usage
```
/pfp [@user]
/pfp
-pfp [@user]
-pfp
```

### Parameters
- `user` (optional): The user whose profile picture you want to see
  - If no user is specified, shows your own profile picture

### Examples

**Get your own profile picture:**
```
/pfp
```

**Get another user's profile picture:**
```
/pfp @JohnDoe
```

**Output:**
The bot will send an embed containing:
- The user's profile picture as a large image
- Title showing "{Username}'s Profile Picture"
- Footer showing who requested it
- Discord's signature blurple color theme

### Features
- **Automatic Fallback:** If a user has no custom avatar, shows their default Discord avatar
- **Rich Embeds:** Displays the image in a beautiful embed format
- **Attribution:** Shows who requested the profile picture in the footer
- **Self-Reference:** Use without parameters to see your own profile picture
- **Both Command Types:** Works as both slash command (`/pfp`) and prefix command (`-pfp`)
- **High Quality:** Gets the full-resolution avatar image

### Technical Notes
- Works with any Discord user in the server
- Automatically handles users without custom avatars
- Supports both current and legacy Discord avatar formats
- Images are displayed at their maximum available resolution

---

## Installation Status

Both commands have been successfully added to the bot:

✅ **coinflip.rs** - Random coin flip simulation
✅ **pfp.rs** - Profile picture display
✅ **Dependencies** - Added `rand` crate for random number generation
✅ **Module Registration** - Commands registered in mod.rs and main.rs
✅ **Tests** - Unit tests included for both commands
✅ **Build Verification** - All code compiles successfully

---

## Animation Technical Details

### Coinflip Animation Implementation
The coinflip command uses Discord's message editing API to create a smooth animation:

1. **Initial Message:** Sends a message saying the user is flipping a coin
2. **Animation Loop:** Edits the message 6 times with alternating coin emojis (🪙/🔄)
3. **Timing:** Each frame displays for 300ms (0.3 seconds)
4. **Final Pause:** 500ms pause before revealing the result
5. **Result:** Final edit shows the actual coin flip outcome

**Total Animation Time:** ~2.3 seconds

**Technical Implementation:**
- Uses `tokio::time::sleep()` for precise timing
- Employs `poise::CreateReply::default().content()` for message editing
- Includes error handling for failed edits
- Generates the actual result before animation starts for consistency

The animation creates an engaging user experience while maintaining the integrity of the random coin flip result.

The commands are now ready to use!