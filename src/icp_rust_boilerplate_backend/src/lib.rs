#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use regex::Regex;
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Property {
    id: u64,
    owner_id: u64,
    address: String,
    description: String,
    created_at: u64,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Owner {
    id: u64,
    name: String,
    email: String,
    created_at: u64,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Transaction {
    id: u64,
    property_id: u64,
    from_owner_id: u64,
    to_owner_id: u64,
    transaction_date: u64,
    created_at: u64,
    amount: u64, // Adding the amount field for money transactions
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct MoneyTransaction {
    id: u64,
    from_owner_id: u64,
    to_owner_id: u64,
    amount: u64,
    transaction_date: u64,
    created_at: u64,
}

impl Storable for Property {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Property {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for Owner {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Owner {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for Transaction {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Transaction {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for MoneyTransaction {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for MoneyTransaction {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static PROPERTIES_STORAGE: RefCell<StableBTreeMap<u64, Property, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static OWNERS_STORAGE: RefCell<StableBTreeMap<u64, Owner, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static TRANSACTIONS_STORAGE: RefCell<StableBTreeMap<u64, Transaction, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));

    static MONEY_TRANSACTIONS_STORAGE: RefCell<StableBTreeMap<u64, MoneyTransaction, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4)))
    ));
}

#[derive(candid::CandidType, Deserialize, Serialize)]
struct PropertyPayload {
    owner_id: u64,
    address: String,
    description: String,
}

#[derive(candid::CandidType, Deserialize, Serialize)]
struct OwnerPayload {
    name: String,
    email: String,
}

#[derive(candid::CandidType, Deserialize, Serialize)]
struct TransactionPayload {
    property_id: u64,
    from_owner_id: u64,
    to_owner_id: u64,
    transaction_date: u64,
    amount: u64, // Adding the amount field to the payload
}

#[derive(candid::CandidType, Deserialize, Serialize)]
struct MoneyTransactionPayload {
    from_owner_id: u64,
    to_owner_id: u64,
    amount: u64,
    transaction_date: u64,
}

#[derive(candid::CandidType, Deserialize, Serialize)]
struct SearchPropertyPayload {
    address: Option<String>,
    owner_id: Option<u64>,
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Message {
    Success(String),
    Error(String),
    NotFound(String),
    InvalidPayload(String),
    Unauthorized(String),
}

#[ic_cdk::update]
fn create_property(payload: PropertyPayload) -> Result<Property, Message> {
    if payload.address.is_empty() || payload.description.is_empty() {
        return Err(Message::InvalidPayload(
            "Ensure 'address' and 'description' are provided.".to_string(),
        ));
    }

    let owner_exists = OWNERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, owner)| owner.id == payload.owner_id)
    });

    if !owner_exists {
        return Err(Message::NotFound("Owner not found".to_string()));
    }

    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let property = Property {
        id,
        owner_id: payload.owner_id,
        address: payload.address,
        description: payload.description,
        created_at: current_time(),
    };
    PROPERTIES_STORAGE.with(|storage| storage.borrow_mut().insert(id, property.clone()));
    Ok(property)
}

#[ic_cdk::update]
fn transfer_property(payload: TransactionPayload) -> Result<Transaction, Message> {
    // Ensure the property exists
    let property_exists = PROPERTIES_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, property)| property.id == payload.property_id)
    });

    if !property_exists {
        return Err(Message::NotFound("Property not found".to_string()));
    }

    // Ensure the from owner exists
    let from_owner_exists = OWNERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, owner)| owner.id == payload.from_owner_id)
    });

    if !from_owner_exists {
        return Err(Message::NotFound("From owner not found".to_string()));
    }

    // Ensure the to owner exists
    let to_owner_exists = OWNERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, owner)| owner.id == payload.to_owner_id)
    });

    if !to_owner_exists {
        return Err(Message::NotFound("To owner not found".to_string()));
    }

    // Ensure the from owner is the current owner of the property
    let is_owner = PROPERTIES_STORAGE.with(|storage| {
        storage.borrow().iter().any(|(_, property)| {
            property.id == payload.property_id && property.owner_id == payload.from_owner_id
        })
    });
    if !is_owner {
        return Err(Message::Unauthorized(
            "Unauthorized to transfer property".to_string(),
        ));
    }

    // Ensure the from owner cannot transfer property to themselves
    if payload.from_owner_id == payload.to_owner_id {
        return Err(Message::Unauthorized(
            "Cannot transfer property to yourself".to_string(),
        ));
    }

    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let transaction = Transaction {
        id,
        property_id: payload.property_id,
        from_owner_id: payload.from_owner_id,
        to_owner_id: payload.to_owner_id,
        transaction_date: payload.transaction_date,
        created_at: current_time(),
        amount: payload.amount, // Set the amount for the transaction
    };

    TRANSACTIONS_STORAGE.with(|storage| storage.borrow_mut().insert(id, transaction.clone()));
    PROPERTIES_STORAGE.with(|storage| {
        if let Some(mut property) = storage.borrow_mut().remove(&payload.property_id) {
            property.owner_id = payload.to_owner_id;
            storage.borrow_mut().insert(payload.property_id, property);
        }
    });

    Ok(transaction)
}

#[ic_cdk::update]
fn create_money_transaction(payload: MoneyTransactionPayload) -> Result<MoneyTransaction, Message> {
    // Ensure the from owner exists
    let from_owner_exists = OWNERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, owner)| owner.id == payload.from_owner_id)
    });

    if !from_owner_exists {
        return Err(Message::NotFound("From owner not found".to_string()));
    }

    // Ensure the to owner exists
    let to_owner_exists = OWNERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, owner)| owner.id == payload.to_owner_id)
    });

    if !to_owner_exists {
        return Err(Message::NotFound("To owner not found".to_string()));
    }

    // Ensure the from owner cannot transfer money to themselves
    if payload.from_owner_id == payload.to_owner_id {
        return Err(Message::Unauthorized(
            "Cannot transfer money to yourself".to_string(),
        ));
    }

    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let money_transaction = MoneyTransaction {
        id,
        from_owner_id: payload.from_owner_id,
        to_owner_id: payload.to_owner_id,
        amount: payload.amount,
        transaction_date: payload.transaction_date,
        created_at: current_time(),
    };

    MONEY_TRANSACTIONS_STORAGE.with(|storage| storage.borrow_mut().insert(id, money_transaction.clone()));

    Ok(money_transaction)
}

#[ic_cdk::query]
fn get_properties() -> Result<Vec<Property>, Message> {
    PROPERTIES_STORAGE.with(|storage| {
        let properties: Vec<Property> = storage
            .borrow()
            .iter()
            .map(|(_, property)| property.clone())
            .collect();

        if properties.is_empty() {
            Err(Message::NotFound("No properties found".to_string()))
        } else {
            Ok(properties)
        }
    })
}

#[ic_cdk::query]
fn get_property_by_id(id: u64) -> Result<Property, Message> {
    PROPERTIES_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .find(|(_, property)| property.id == id)
            .map(|(_, property)| property.clone())
            .ok_or(Message::NotFound("Property not found".to_string()))
    })
}

#[ic_cdk::query]
fn search_properties(payload: SearchPropertyPayload) -> Result<Vec<Property>, Message> {
    PROPERTIES_STORAGE.with(|storage| {
        let properties: Vec<Property> = storage
            .borrow()
            .iter()
            .filter(|(_, property)| {
                if let Some(address) = &payload.address {
                    if !property.address.contains(address) {
                        return false;
                    }
                }
                if let Some(owner_id) = payload.owner_id {
                    if property.owner_id != owner_id {
                        return false;
                    }
                }
                true
            })
            .map(|(_, property)| property.clone())
            .collect();

        if properties.is_empty() {
            Err(Message::NotFound("No properties found".to_string()))
        } else {
            Ok(properties)
        }
    })
}

#[ic_cdk::update]
fn create_owner(payload: OwnerPayload) -> Result<Owner, Message> {
    if payload.name.is_empty() || payload.email.is_empty() {
        return Err(Message::InvalidPayload(
            "Ensure 'name' and 'email' are provided.".to_string(),
        ));
    }

    // Validate the email address format
    let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    if !email_regex.is_match(&payload.email) {
        return Err(Message::InvalidPayload(
            "Invalid email address format".to_string(),
        ));
    }

    // Ensure each email is unique
    let email_exists = OWNERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, owner)| owner.email == payload.email)
    });
    if email_exists {
        return Err(Message::InvalidPayload("Email already exists".to_string()));
    }

    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let owner = Owner {
        id,
        name: payload.name,
        email: payload.email,
        created_at: current_time(),
    };
    OWNERS_STORAGE.with(|storage| storage.borrow_mut().insert(id, owner.clone()));
    Ok(owner)
}

#[ic_cdk::query]
fn get_owners() -> Result<Vec<Owner>, Message> {
    OWNERS_STORAGE.with(|storage| {
        let owners: Vec<Owner> = storage
            .borrow()
            .iter()
            .map(|(_, owner)| owner.clone())
            .collect();

        if owners.is_empty() {
            Err(Message::NotFound("No owners found".to_string()))
        } else {
            Ok(owners)
        }
    })
}

#[ic_cdk::query]
fn get_owner_by_id(id: u64) -> Result<Owner, Message> {
    OWNERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .find(|(_, owner)| owner.id == id)
            .map(|(_, owner)| owner.clone())
            .ok_or(Message::NotFound("Owner not found".to_string()))
    })
}

#[ic_cdk::query]
fn get_transactions() -> Result<Vec<Transaction>, Message> {
    TRANSACTIONS_STORAGE.with(|storage| {
        let transactions: Vec<Transaction> = storage
            .borrow()
            .iter()
            .map(|(_, transaction)| transaction.clone())
            .collect();

        if transactions.is_empty() {
            Err(Message::NotFound("No transactions found".to_string()))
        } else {
            Ok(transactions)
        }
    })
}

#[ic_cdk::query]
fn get_transaction_by_id(id: u64) -> Result<Transaction, Message> {
    TRANSACTIONS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .find(|(_, transaction)| transaction.id == id)
            .map(|(_, transaction)| transaction.clone())
            .ok_or(Message::NotFound("Transaction not found".to_string()))
    })
}

#[ic_cdk::query]
fn get_money_transactions() -> Result<Vec<MoneyTransaction>, Message> {
    MONEY_TRANSACTIONS_STORAGE.with(|storage| {
        let transactions: Vec<MoneyTransaction> = storage
            .borrow()
            .iter()
            .map(|(_, transaction)| transaction.clone())
            .collect();

        if transactions.is_empty() {
            Err(Message::NotFound("No money transactions found".to_string()))
        } else {
            Ok(transactions)
        }
    })
}

#[ic_cdk::query]
fn get_money_transaction_by_id(id: u64) -> Result<MoneyTransaction, Message> {
    MONEY_TRANSACTIONS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .find(|(_, transaction)| transaction.id == id)
            .map(|(_, transaction)| transaction.clone())
            .ok_or(Message::NotFound("Money transaction not found".to_string()))
    })
}

fn current_time() -> u64 {
    time()
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    UnAuthorized { msg: String },
}

ic_cdk::export_candid!();
