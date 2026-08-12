use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::FunctionValue;
use inkwell::AddressSpace;

/// Declares external runtime functions (malloc, free, printf) in the LLVM module.
pub struct Builtins<'ctx> {
    pub malloc_fn: FunctionValue<'ctx>,
    pub free_fn: FunctionValue<'ctx>,
    pub printf_fn: FunctionValue<'ctx>,
}

impl<'ctx> Builtins<'ctx> {
    pub fn declare(context: &'ctx Context, module: &Module<'ctx>) -> Self {
        let ptr_type = context.ptr_type(AddressSpace::default());
        let i64_type = context.i64_type();
        let i32_type = context.i32_type();
        let void_type = context.void_type();

        // void* malloc(size_t size)
        let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
        let malloc_fn = module.add_function(
            "malloc",
            malloc_type,
            Some(inkwell::module::Linkage::External),
        );

        // void free(void* ptr)
        let free_type = void_type.fn_type(&[ptr_type.into()], false);
        let free_fn =
            module.add_function("free", free_type, Some(inkwell::module::Linkage::External));

        // int printf(const char* fmt, ...)
        let printf_type = i32_type.fn_type(&[ptr_type.into()], true);
        let printf_fn = module.add_function(
            "printf",
            printf_type,
            Some(inkwell::module::Linkage::External),
        );

        Self {
            malloc_fn,
            free_fn,
            printf_fn,
        }
    }
}
