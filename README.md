# field_trip_fund

## Project Title
field_trip_fund

## Project Description
`field_trip_fund` is a Soroban smart contract that lets a class pool money for a school field trip. A teacher opens a fundraising pool for a specific trip with a target amount, parents and students contribute toward that target, and the teacher records milestone expenses (bus, tickets, lunch, museum entry) as the trip is planned. Every contribution and every milestone is stored on-chain, giving the class a transparent, tamper-proof ledger of how the trip was funded and how the funds were spent.

Unlike a one-to-one scholarship grant, this dApp is intentionally designed for *class-level crowdfunding*: many small contributions from many families roll up into a single shared pool, then a designated teacher releases the money against itemized milestones. This MVP focuses on the storage and authorization logic; it does not move real XLM.

## Project Vision
Our long-term vision is to make school fundraising transparent and trustworthy for every stakeholder. Today, field-trip money is collected in envelopes, cash apps, or spreadsheets, and parents rarely know how the total was spent. By moving the pool to a public blockchain, the dApp aims to become the default coordination layer for school, club, and youth-group trips — eventually supporting recurring trip templates, partial milestone approvals from a parent committee, and integration with mobile money rails in regions where schools cannot easily accept bank transfers.

## Key Features
- **Trip pool creation** — a teacher opens a new trip with a unique `trip_id`, a destination, and a target amount; only the teacher can later close the trip or log milestones.
- **Many-to-one contributions** — parents, students, and sponsors can each contribute any positive amount; the contract tracks a per-contributor running total so the class can see who gave what.
- **Milestone-based expense logging** — the teacher records named milestones (e.g. `bus`, `tickets`, `lunch`) with their cost; the contract refuses any milestone whose cost would exceed the amount raised so far.
- **Read-only views** — `raised`, `spent`, `target`, `destination`, `teacher`, `is_closed`, `contribution_count`, `milestone_count`, and `contribution_of` make it trivial for parents or a frontend to display live status without sending a transaction.
- **Auth-gated state changes** — every state-changing function uses Soroban `require_auth()` so no one can move funds or log milestones on someone else's behalf, and contributions are blocked the moment the teacher closes the pool.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** education dApp — see `contracts/field_trip_fund/src/lib.rs` for the full field_trip_fund business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `CCSGCLZ7X7JGO5FP47QTMOXQPXBZQ7LW7BIFCSDXOPVXCU52BK5YTTRZ`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/a759d89627df281e703c95eba2075869e09e02267dc5ba247e7f9dfdeba70592`


## Future Scope
- **Move real value** — wire the contract to a Stellar asset (XLM or a custom school token) so `contribute` and `mark_milestone` actually transfer and release funds, with the contract acting as a milestone-based escrow.
- **Multi-signer milestones** — require approval from a small parent committee before a milestone cost above a configurable threshold is accepted, turning the contract into a lightweight DAO.
- **Per-trip templates and history** — let teachers clone a previous trip's structure (route, default milestones) and emit on-chain events so a parent-facing app can show a live "thermometer" of progress and a full spending history.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `field_trip_fund` (education)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
