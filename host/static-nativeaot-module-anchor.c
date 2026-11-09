#if !defined(__has_attribute)
#error "The Linux static host requires a compiler that supports __has_attribute."
#endif
#if !__has_attribute(retain)
#error "The Linux static host requires a compiler that supports the retain attribute."
#endif

extern const char __Module;

/* LLD's section GC does not retain __modules through __start/__stop alone. */
__attribute__((used, retain))
static const void *const rustolonia_nativeaot_module_anchor = &__Module;
