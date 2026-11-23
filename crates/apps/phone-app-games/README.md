# Games and independent game apps

`phone-app-games` renders the collection/player hub. The collection discovers
registered manifests with `AppCategory::Game`; it has no dependency on a game
implementation. Activating a collection row uses that manifest's route. The
composition root registers each game separately, just like a utility or a
built-in application.

The included games are local SDK examples. They do not connect to Xbox accounts,
friends, online achievements or leaderboards, and they are not represented as
games originally bundled with Windows Phone.

## Tic tac toe

`phone-app-tic-tac-toe` owns its bounded move-history model, minimax player and
native board. It supports a perfect phone opponent and two people sharing one
device. Undo removes an unfinished turn, including the phone's reply. Completed
games increment the appropriate saved score once and cannot be undone to alter
that result. New game and switching modes preserve accumulated scores.

Move histories are validated when restored: indices, alternation, occupied cells,
moves after a terminal result and the phone's replies must be consistent. A bad
save exposes a recovery command instead of indexing an invalid board.

## Minesweeper

`phone-app-minesweeper` owns three bounded board sizes: 8 × 8 with 10 mines,
10 × 10 with 18 mines and 12 × 12 with 30 mines. A deterministic generator places
mines only after the first reveal; that square and its neighbours are safe.

Use **flag** to mark squares and **reveal** to return to uncovering them. A revealed
number performs a chord when the number of adjacent flags matches it. Incorrect
flags can therefore lose the game. Zero regions flood-fill, and clearing every
safe square completes the game. Each difficulty records wins, losses and best
elapsed time. The elapsed clock is wall time from first reveal to completion;
leaving or reloading the app does not pause that time.

Mine placement, adjacency, board size, flags, first reveal and outcome are
validated on every restored model. Completed results are recorded once.

## SDK contracts exercised

Both games register with `AppManifest` and `AppRegistration`, render declarative
FerroUI controls through `AppContext`, and use bounded `AppStorage` namespaces.
Neither imports the system-shell crate, another game's implementation, or the
hub. Boards and scores are part of the normal phone document and backup.

Minesweeper keeps its flag mode and clock in its own typed session state. The SDK
tick callback updates only that clock; it does not save the phone document. The
reset hook clears transient state after phone restoration. Native screen-reader
labels identify each cell's row, column and visible state.

## Verification

The engine tests explore every human move sequence against the phone player and
exercise terminal-score handling, undo, first-click safety at every board
position, deterministic minefields, flooding, chording, flags and corrupt saves.

The opt-in `verifySdkGames` browser module uses trusted native input to launch
registered games, finish a local match, undo a phone turn, reveal/flag a minefield
and verify reload/backup roundtrips. It also checks that the visible game clock
advances without writing phone data. Results belong to the CI run that executes
the module; source presence alone is not a passing visual verification.

Original Games Hub references: Microsoft's [Windows Phone 8 Games Hub overview](https://blogs.windows.com/windowsexperience/2012/12/12/whats-new-for-gamers-in-windows-phone-8/)
and [downloadable classic games announcement](https://blogs.windows.com/windowsexperience/2013/12/27/microsoft-gaming-classics-solitaire-mahjong-and-minesweeper-arrive-on-windows-phone/).
