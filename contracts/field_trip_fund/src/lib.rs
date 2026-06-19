#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Map, Symbol, Vec};

// Storage key prefixes. Each trip is namespaced by its `trip_id` Symbol so a
// single contract instance can host many independent field-trip pools.
const K_TEACHER: &str = "teacher";
const K_DEST: &str = "dest";
const K_TARGET: &str = "target";
const K_RAISED: &str = "raised";
const K_SPENT: &str = "spent";
const K_CLOSED: &str = "closed";
const K_NCONTRIB: &str = "ncontrib";
const K_NMILEST: &str = "nmilest";
const K_CONTRIB: &str = "contrib";
const K_MLIST: &str = "mlist";
const K_MCOST: &str = "mcost";

/// Build a composite storage key `(prefix, trip_id)` for a given trip.
/// Tuples are valid Soroban storage keys and avoid name collisions
/// across many trips stored in the same contract instance.
fn key(prefix: &str, trip: &Symbol, env: &Env) -> (Symbol, Symbol) {
    (Symbol::new(env, prefix), trip.clone())
}

/// `FieldTripFund` is a class-level crowdfunding pool for school field trips.
/// A teacher opens a trip with a target amount, parents and students contribute
/// toward that target, and the teacher records milestone expenses as the trip
/// progresses. Unlike a one-to-one scholarship grant, every contribution and
/// every milestone is recorded on-chain so the class can audit how the money
/// is raised and spent.
///
/// This contract intentionally does NOT move real XLM. It is a storage- and
/// logic-focused MVP that demonstrates Soroban auth, storage, and event
/// patterns. A future iteration can plug in the Stellar asset / token client
/// to actually move funds.
#[contract]
pub struct FieldTripFund;

#[contractimpl]
impl FieldTripFund {
    /// Open a new field-trip fundraising pool.
    ///
    /// The `teacher` is the only address allowed to mark milestones or close
    /// the trip. `trip_id` must be unique per deployment. `target_amount` is
    /// the goal the class is crowdfunding toward, denominated in the smallest
    /// unit of the chosen asset (e.g. stroops for XLM).
    pub fn create_trip(
        env: Env,
        teacher: Address,
        trip_id: Symbol,
        destination: Symbol,
        target_amount: u64,
    ) {
        // Teacher authorizes the creation of a new pool under their name.
        teacher.require_auth();

        if target_amount == 0 {
            panic!("Target amount must be greater than zero");
        }
        if env.storage().instance().has(&key(K_TEACHER, &trip_id, &env)) {
            panic!("Trip already exists for this id");
        }

        env.storage()
            .instance()
            .set(&key(K_TEACHER, &trip_id, &env), &teacher);
        env.storage()
            .instance()
            .set(&key(K_DEST, &trip_id, &env), &destination);
        env.storage()
            .instance()
            .set(&key(K_TARGET, &trip_id, &env), &target_amount);
        env.storage()
            .instance()
            .set(&key(K_RAISED, &trip_id, &env), &0u64);
        env.storage()
            .instance()
            .set(&key(K_SPENT, &trip_id, &env), &0u64);
        env.storage()
            .instance()
            .set(&key(K_CLOSED, &trip_id, &env), &false);
        env.storage()
            .instance()
            .set(&key(K_NCONTRIB, &trip_id, &env), &0u32);
        env.storage()
            .instance()
            .set(&key(K_NMILEST, &trip_id, &env), &0u32);
    }

    /// Record a contribution from a parent, student, or sponsor into the
    /// trip's fund. The `contributor` authorizes their own payment. The
    /// caller's per-trip running total is tracked in a `Map<Address, u64>`
    /// so the class can later see who gave what.
    ///
    /// Returns the new total amount raised for the trip after this
    /// contribution is applied.
    pub fn contribute(
        env: Env,
        contributor: Address,
        trip_id: Symbol,
        amount: u64,
    ) -> u64 {
        // Contributor authorizes moving value on their behalf.
        contributor.require_auth();

        if amount == 0 {
            panic!("Contribution must be greater than zero");
        }
        if env
            .storage()
            .instance()
            .get::<_, bool>(&key(K_CLOSED, &trip_id, &env))
            .unwrap_or(false)
        {
            panic!("Trip fund is closed");
        }
        // Make sure the trip exists by checking the teacher slot.
        if !env.storage().instance().has(&key(K_TEACHER, &trip_id, &env)) {
            panic!("Trip not found");
        }

        let raised: u64 = env
            .storage()
            .instance()
            .get(&key(K_RAISED, &trip_id, &env))
            .unwrap_or(0u64);
        let new_raised = raised
            .checked_add(amount)
            .expect("Contribution overflows u64");
        env.storage()
            .instance()
            .set(&key(K_RAISED, &trip_id, &env), &new_raised);

        // Bump contribution counter.
        let n: u32 = env
            .storage()
            .instance()
            .get(&key(K_NCONTRIB, &trip_id, &env))
            .unwrap_or(0u32);
        env.storage()
            .instance()
            .set(&key(K_NCONTRIB, &trip_id, &env), &(n + 1));

        // Track per-contributor total in a Map<Address, u64>.
        let ckey = key(K_CONTRIB, &trip_id, &env);
        let mut contribs: Map<Address, u64> = env
            .storage()
            .instance()
            .get(&ckey)
            .unwrap_or(Map::new(&env));
        let prev = contribs.get(contributor.clone()).unwrap_or(0u64);
        contribs.set(contributor, prev + amount);
        env.storage().instance().set(&ckey, &contribs);

        new_raised
    }

    /// Record a milestone expense against the trip fund. Only the original
    /// trip teacher may call this, and the milestone cost may not exceed the
    /// amount currently raised. The milestone's name and cost are appended
    /// to per-trip logs so the class can audit spending.
    ///
    /// Returns the 1-indexed position of the new milestone in the trip's
    /// milestone log.
    pub fn mark_milestone(
        env: Env,
        teacher: Address,
        trip_id: Symbol,
        milestone: Symbol,
        cost: u64,
    ) -> u32 {
        teacher.require_auth();

        let stored_teacher: Address = env
            .storage()
            .instance()
            .get(&key(K_TEACHER, &trip_id, &env))
            .expect("Trip not found");
        if stored_teacher != teacher {
            panic!("Only the trip teacher can mark milestones");
        }
        if env
            .storage()
            .instance()
            .get::<_, bool>(&key(K_CLOSED, &trip_id, &env))
            .unwrap_or(false)
        {
            panic!("Trip fund is closed");
        }
        if cost == 0 {
            panic!("Milestone cost must be greater than zero");
        }

        let raised: u64 = env
            .storage()
            .instance()
            .get(&key(K_RAISED, &trip_id, &env))
            .unwrap_or(0u64);
        let spent: u64 = env
            .storage()
            .instance()
            .get(&key(K_SPENT, &trip_id, &env))
            .unwrap_or(0u64);
        if spent
            .checked_add(cost)
            .expect("Milestone total overflows u64")
            > raised
        {
            panic!("Milestone cost exceeds funds raised");
        }

        env.storage()
            .instance()
            .set(&key(K_SPENT, &trip_id, &env), &(spent + cost));

        let n: u32 = env
            .storage()
            .instance()
            .get(&key(K_NMILEST, &trip_id, &env))
            .unwrap_or(0u32);
        let new_n = n + 1;
        env.storage()
            .instance()
            .set(&key(K_NMILEST, &trip_id, &env), &new_n);

        // Append milestone name and cost to per-trip logs.
        let lkey = key(K_MLIST, &trip_id, &env);
        let mut names: Vec<Symbol> = env
            .storage()
            .instance()
            .get(&lkey)
            .unwrap_or(Vec::new(&env));
        names.push_back(milestone.clone());
        env.storage().instance().set(&lkey, &names);

        let ckey = key(K_MCOST, &trip_id, &env);
        let mut costs: Vec<u64> = env
            .storage()
            .instance()
            .get(&ckey)
            .unwrap_or(Vec::new(&env));
        costs.push_back(cost);
        env.storage().instance().set(&ckey, &costs);

        new_n
    }

    /// Close the trip fund. Only the original trip teacher may close it.
    /// After closing, no further contributions or milestones are accepted.
    /// Returns `true` once the trip is marked closed.
    pub fn close_trip(env: Env, teacher: Address, trip_id: Symbol) -> bool {
        teacher.require_auth();

        let stored_teacher: Address = env
            .storage()
            .instance()
            .get(&key(K_TEACHER, &trip_id, &env))
            .expect("Trip not found");
        if stored_teacher != teacher {
            panic!("Only the trip teacher can close the trip");
        }
        env.storage()
            .instance()
            .set(&key(K_CLOSED, &trip_id, &env), &true);
        true
    }

    // ---------------- Read-only views ----------------

    /// Returns the total amount raised for a given trip.
    pub fn raised(env: Env, trip_id: Symbol) -> u64 {
        env.storage()
            .instance()
            .get(&key(K_RAISED, &trip_id, &env))
            .unwrap_or(0u64)
    }

    /// Returns the total amount spent (sum of all milestone costs) for a trip.
    pub fn spent(env: Env, trip_id: Symbol) -> u64 {
        env.storage()
            .instance()
            .get(&key(K_SPENT, &trip_id, &env))
            .unwrap_or(0u64)
    }

    /// Returns the target amount that was set when the trip was created.
    pub fn target(env: Env, trip_id: Symbol) -> u64 {
        env.storage()
            .instance()
            .get(&key(K_TARGET, &trip_id, &env))
            .unwrap_or(0u64)
    }

    /// Returns the destination that was set when the trip was created.
    pub fn destination(env: Env, trip_id: Symbol) -> Symbol {
        env.storage()
            .instance()
            .get(&key(K_DEST, &trip_id, &env))
            .expect("Trip not found")
    }

    /// Returns the teacher (admin) address that owns the trip.
    pub fn teacher(env: Env, trip_id: Symbol) -> Address {
        env.storage()
            .instance()
            .get(&key(K_TEACHER, &trip_id, &env))
            .expect("Trip not found")
    }

    /// Returns `true` if the trip fund has been closed by the teacher.
    pub fn is_closed(env: Env, trip_id: Symbol) -> bool {
        env.storage()
            .instance()
            .get(&key(K_CLOSED, &trip_id, &env))
            .unwrap_or(false)
    }

    /// Returns the number of contributions received for the trip.
    pub fn contribution_count(env: Env, trip_id: Symbol) -> u32 {
        env.storage()
            .instance()
            .get(&key(K_NCONTRIB, &trip_id, &env))
            .unwrap_or(0u32)
    }

    /// Returns the number of milestones recorded for the trip.
    pub fn milestone_count(env: Env, trip_id: Symbol) -> u32 {
        env.storage()
            .instance()
            .get(&key(K_NMILEST, &trip_id, &env))
            .unwrap_or(0u32)
    }

    /// Returns the running total a single contributor has given to a trip.
    pub fn contribution_of(env: Env, trip_id: Symbol, contributor: Address) -> u64 {
        let contribs: Map<Address, u64> = env
            .storage()
            .instance()
            .get(&key(K_CONTRIB, &trip_id, &env))
            .unwrap_or(Map::new(&env));
        contribs.get(contributor).unwrap_or(0u64)
    }
}
