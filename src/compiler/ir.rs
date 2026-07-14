use alloc::vec::Vec;
use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    LoadImm(i64),
    LoadImmFloat(f32),
    Add(ValueId, ValueId),
    Sub(ValueId, ValueId),
    Mul(ValueId, ValueId),
    Div(ValueId, ValueId),
    
    // SSA specific operations
    Phi(Vec<(ValueId, usize)>), // Block id
    
    // FFI and External Calls
    FfiCall {
        func_id: u32,
        args: Vec<ValueId>,
    },
    
    // DB / Tensor intrinsic fusion before lowering
    TensorMul(ValueId, ValueId),
    DbFilter(ValueId, String), // simplified representation
    
    // Return
    Return(Option<ValueId>),
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub id: ValueId,
    pub op: Op,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: usize,
    pub insts: Vec<Instruction>,
    pub successors: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct FunctionIR {
    pub name: String,
    pub blocks: Vec<BasicBlock>,
    pub next_value_id: usize,
}

impl FunctionIR {
    pub fn new(name: String) -> Self {
        Self {
            name,
            blocks: Vec::new(),
            next_value_id: 0,
        }
    }

    pub fn alloc_val(&mut self) -> ValueId {
        let id = self.next_value_id;
        self.next_value_id += 1;
        ValueId(id)
    }
}
