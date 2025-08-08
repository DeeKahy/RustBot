# 🎮 Discord Bot Games

This Discord bot includes three fun interactive games you can play directly in Discord!

## 🎯 Number Guessing Game

Guess a randomly generated number with helpful hints!

### Commands:
- `/numberguess [min] [max]` - Start a new game (default range: 1-100)
- `/guess <number>` - Make a guess
- `/hint` - Get helpful hints about your game
- `/gamestatus` - Check your current game progress
- `/endgame` - End your current game

### How to Play:
1. Use `/numberguess` to start (optionally specify custom range)
2. The bot will think of a number in the specified range
3. Use `/guess <number>` to make guesses
4. Get "too high" or "too low" hints with proximity feedback
5. Try to guess in as few attempts as possible!

### Features:
- 🔥 Proximity hints (hot/cold feedback)
- 📊 Performance rating system
- 🎯 Custom number ranges
- 📈 Attempt tracking

---

## ⭕ Tic-Tac-Toe

Classic 3x3 grid game with two-player mode or AI opponent!

### Commands:
- `/tictactoe [@opponent]` - Start a game (vs player or AI if no opponent)
- `/move <position>` - Make your move (position 1-9)
- `/board` - View current game board
- `/endttt` - End your current game

### How to Play:
1. Use `/tictactoe` to play vs AI, or `/tictactoe @friend` for two-player
2. X always goes first
3. Use `/move <number>` where number is 1-9:
   ```
   1 | 2 | 3
   --|---|--
   4 | 5 | 6
   --|---|--
   7 | 8 | 9
   ```
4. Get three in a row to win!

### Features:
- 🤖 Smart AI opponent with strategic play
- 👥 Two-player multiplayer support
- 🎨 Clean ASCII board display
- 🏆 Win detection for all patterns

---

## 🎪 Hangman

Classic word guessing game with programming-themed words!

### Commands:
- `/hangman [custom_word]` - Start a new game
- `/letter <letter>` - Guess a letter
- `/hangmanstatus` - Check your game progress
- `/hangmanhint` - Get hints about the word
- `/endhangman` - End your current game

### How to Play:
1. Use `/hangman` to start with a random word
2. Use `/letter <letter>` to guess letters one at a time
3. Wrong guesses add body parts to the hangman drawing
4. Guess the complete word before the drawing is finished!

### Features:
- 🎨 ASCII hangman drawing that progresses with wrong guesses
- 📚 Programming and tech-themed word categories
- 💡 Hint system showing category and progress
- 🎯 Custom word support for practice

### Word Categories:
- Programming Languages (Python, Rust, JavaScript)
- Programming Concepts (Function, Loop, Array)
- Data Structures (Vector, HashMap)
- Discord/Tech Terms (Server, Channel, Command)
- General Knowledge words

---

## 🎮 General Game Features

### Session Management:
- Each player can have one active game per game type
- Games are automatically saved during play
- Use the respective "end" commands to quit early

### User Experience:
- 🎨 Rich formatting with emojis and Discord markdown
- 📊 Progress tracking and statistics
- 🎯 Helpful error messages and guidance
- 🏆 Performance ratings and achievements

### Fair Play:
- Games are isolated per user (no interference)
- AI difficulty is balanced for fun gameplay
- Random elements ensure replayability

---

## 🚀 Quick Start

Try these commands to get started:

```
/numberguess           # Start number guessing (1-100)
/tictactoe            # Play tic-tac-toe vs AI
/hangman              # Start hangman with random word
```

Have fun gaming! 🎉