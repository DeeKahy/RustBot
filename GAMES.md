# Discord Bot Games

RustBot includes three interactive games you can play directly in Discord. Every command works with
both the `-` prefix and the `/` slash form.

## Number Guessing

Guess a randomly generated number with hints.

### Commands
- `-numberguess [min] [max]` - Start a new game (default range 1-100)
- `-guess <number>` - Make a guess
- `-hint` - Get a hint about your game
- `-gamestatus` - Check your current progress
- `-endgame` - End your current game

### How to play
1. Start with `-numberguess` (optionally pass a custom range).
2. The bot picks a number in the range.
3. Guess with `-guess <number>` and use the "too high" / "too low" and proximity feedback.
4. Try to find it in as few attempts as possible.

### Features
- Proximity (hot/cold) hints
- Performance rating and attempt tracking
- Custom number ranges

---

## Tic-Tac-Toe

Classic 3x3 game, two players or against the AI.

### Commands
- `-tictactoe [@opponent]` - Start a game (vs the AI if no opponent is given)
- `-move_ttt <position>` - Make a move (position 1-9)
- `-board` - View the current board
- `-endttt` - End your current game

### How to play
1. `-tictactoe` plays against the AI; `-tictactoe @friend` starts a two-player game.
2. X always goes first.
3. Move with `-move_ttt <n>`, where positions are numbered:
   ```
   1 | 2 | 3
   --|---|--
   4 | 5 | 6
   --|---|--
   7 | 8 | 9
   ```
4. Get three in a row to win.

### Features
- Strategic AI opponent
- Two-player support
- ASCII board display and full win detection

---

## Hangman

Word guessing game with programming-themed words.

### Commands
- `-hangman [custom_word]` - Start a new game
- `-letter <letter>` - Guess a letter
- `-hangmanstatus` - Check your progress
- `-hangmanhint` - Get a hint about the word
- `-endhangman` - End your current game

### How to play
1. `-hangman` starts with a random word.
2. Guess letters one at a time with `-letter <letter>`.
3. Wrong guesses add to the hangman drawing.
4. Complete the word before the drawing is finished.

### Features
- ASCII drawing that progresses with wrong guesses
- Hint system showing category and progress
- Custom word support for practice

### Word categories
- Programming languages (Python, Rust, JavaScript)
- Programming concepts (Function, Loop, Array)
- Data structures (Vector, HashMap)
- Discord / tech terms (Server, Channel, Command)
- General knowledge

---

## General

- Each player can have one active game per game type.
- Games are saved during play; use the matching "end" command to quit early.
- Games are isolated per user, and the AI is tuned for fun rather than to be unbeatable.

## Quick start

```
-numberguess     # start number guessing (1-100)
-tictactoe       # play tic-tac-toe vs the AI
-hangman         # start hangman with a random word
```
