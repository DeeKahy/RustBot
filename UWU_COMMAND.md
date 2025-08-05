# UwU Command Documentation

## Overview
The `uwu` command transforms text into cute "uwu language" commonly used in anime/manga communities and internet culture.

## Usage

### Method 1: Direct Text Input
Use the command with text directly:
```
/uwu Hello world! This is a test message.
-uwu Hello world! This is a test message.
```

### Method 2: Reply to Messages (Prefix Command Only)
Reply to any message with the prefix command to uwuify that message:
1. Find a message you want to uwuify
2. Reply to it with `-uwu` (no additional text needed)
3. The bot will transform the original message content

**Note:** Message replies only work with prefix commands (`-uwu`), not slash commands (`/uwu`).

## Examples

### Input:
```
The cat loves running around the house
```

### Output:
```
Teh cat uwu wuvs wunnying awound teh house uwu
```

## Transformation Rules

The uwu command applies several transformations:

1. **Basic substitutions:**
   - `r` → `w`
   - `l` → `w`
   - `th` → `d`
   - `The`/`the` → `Teh`/`teh`

2. **Nya patterns:**
   - `na` → `nya`
   - `ne` → `nye`
   - `ni` → `nyi`
   - `no` → `nyo`
   - `nu` → `nyu`

3. **Other patterns:**
   - `ove` → `uv`

4. **Expressions:**
   - Adds cute expressions like `uwu`, `owo`, `>w<`, `^w^`, `(>ω<)` throughout longer text
   - Always ends with `uwu` if not already present

## Features

- **Smart Expression Insertion:** Longer sentences get cute expressions inserted in the middle
- **Reply Attribution:** When replying to someone else's message, shows "*[Username] says:*" before the uwuified text
- **Both Command Types:** Works as both slash command (`/uwu`) and prefix command (`-uwu`)
- **Error Handling:** Provides helpful error messages for empty text or missing input

## Technical Notes

- The command preserves some capitalization patterns
- Works with both prefix commands (`-`) and slash commands (`/`)
- Message replies only work with prefix commands due to Discord API limitations
- Includes comprehensive unit tests to ensure transformation quality