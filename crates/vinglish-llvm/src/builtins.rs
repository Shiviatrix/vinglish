use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::FunctionValue;

/// Declares external runtime functions (malloc, free, printf) in the LLVM module.
pub struct Builtins<'ctx> {
    pub malloc_fn: FunctionValue<'ctx>,
    pub realloc_fn: FunctionValue<'ctx>,
    pub free_fn: FunctionValue<'ctx>,
    pub printf_fn: FunctionValue<'ctx>,

    pub rt_list_new: FunctionValue<'ctx>,
    pub rt_list_get: FunctionValue<'ctx>,
    pub rt_list_borrow_get: FunctionValue<'ctx>,
    pub rt_list_set: FunctionValue<'ctx>,
    pub rt_list_len: FunctionValue<'ctx>,
    pub rt_list_push: FunctionValue<'ctx>,
    pub rt_list_pop: FunctionValue<'ctx>,

    pub ving_sys_env: FunctionValue<'ctx>,
    pub ving_sys_exec: FunctionValue<'ctx>,

    pub ving_regex_is_match: FunctionValue<'ctx>,
    pub ving_regex_replace: FunctionValue<'ctx>,
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

        // void* realloc(void* ptr, size_t size)
        let realloc_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
        let realloc_fn = module.add_function(
            "realloc",
            realloc_type,
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

        // List runtime functions
        let rt_list_new_type = i64_type.fn_type(&[i64_type.into()], false);
        let rt_list_new = module.add_function("rt_list_new", rt_list_new_type, Some(inkwell::module::Linkage::External));

        let rt_list_get_type = i64_type.fn_type(&[i64_type.into(), i64_type.into()], false);
        let rt_list_get = module.add_function("rt_list_get", rt_list_get_type, Some(inkwell::module::Linkage::External));
        let rt_list_borrow_get = module.add_function("rt_list_borrow_get", rt_list_get_type, Some(inkwell::module::Linkage::External));

        let rt_list_set_type = void_type.fn_type(&[i64_type.into(), i64_type.into(), i64_type.into()], false);
        let rt_list_set = module.add_function("rt_list_set", rt_list_set_type, Some(inkwell::module::Linkage::External));

        let rt_list_len_type = i64_type.fn_type(&[i64_type.into()], false);
        let rt_list_len = module.add_function("rt_list_len", rt_list_len_type, Some(inkwell::module::Linkage::External));

        let rt_list_push_type = void_type.fn_type(&[i64_type.into(), i64_type.into()], false);
        let rt_list_push = module.add_function("rt_list_push", rt_list_push_type, Some(inkwell::module::Linkage::External));

        let rt_list_pop_type = i64_type.fn_type(&[i64_type.into()], false);
        let rt_list_pop = module.add_function("rt_list_pop", rt_list_pop_type, Some(inkwell::module::Linkage::External));

        // Sys runtime functions
        let ving_sys_env_type = ptr_type.fn_type(&[ptr_type.into()], false);
        let ving_sys_env = module.add_function("ving_sys_env", ving_sys_env_type, Some(inkwell::module::Linkage::External));

        let ving_sys_exec_type = ptr_type.fn_type(&[ptr_type.into()], false);
        let ving_sys_exec = module.add_function("ving_sys_exec", ving_sys_exec_type, Some(inkwell::module::Linkage::External));

        // Regex runtime functions
        let ving_regex_is_match_type = i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        let ving_regex_is_match = module.add_function("ving_regex_is_match", ving_regex_is_match_type, Some(inkwell::module::Linkage::External));

        let ving_regex_replace_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false);
        let ving_regex_replace = module.add_function("ving_regex_replace", ving_regex_replace_type, Some(inkwell::module::Linkage::External));

        Self {
            malloc_fn,
            realloc_fn,
            free_fn,
            printf_fn,
            rt_list_new,
            rt_list_get,
            rt_list_borrow_get,
            rt_list_set,
            rt_list_len,
            rt_list_push,
            rt_list_pop,
            ving_sys_env,
            ving_sys_exec,
            ving_regex_is_match,
            ving_regex_replace,
        }
    }
}
