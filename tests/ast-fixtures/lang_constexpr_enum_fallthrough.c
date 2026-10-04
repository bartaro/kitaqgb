enum class Mode {
    MODE_IDLE = 0,
    MODE_RUN = 1,
    MODE_STOP = 2,
};

constexpr enum Mode DEFAULT_MODE = MODE_RUN;
constexpr u8 LOOP_LIMIT = 6;

static_assert(DEFAULT_MODE == MODE_RUN, "constexpr_enum_mode");
static_assert(LOOP_LIMIT == 6, "constexpr_u8");

u8 gbuf[LOOP_LIMIT];
u8 sink_mode;
u8 sink_count;

void apply_mode(enum Mode m) {
    if (m == MODE_RUN) {
        sink_mode = 1;
    } else {
        sink_mode = 0;
    }
}

void main() {
    enum Mode mode;
    u8 i;

    mode = DEFAULT_MODE;
    apply_mode(mode);

    i = 0;
    while (i < 4) {
        switch (mode) {
            case MODE_IDLE:
                sink_count = 0;
                break;
            case MODE_RUN:
                sink_count = sink_count + 1;
                fallthrough;
            case MODE_STOP:
                sink_mode = 2;
                break;
            default:
                sink_mode = 3;
                break;
        }
        i++;
    }

    while (1) {
    }
}
