/*
 * C shim that exposes mruby's in-memory bytecode compiler to Rust over FFI.
 *
 * This file does not contain code copied from any third-party project; it only
 * calls mruby's public C API. Memory ownership across the FFI boundary:
 *   - Buffers returned through `out_buf` / `err_msg` are allocated with malloc()
 *     and MUST be released by the caller via `openblink_mrbc_free`.
 */
#include <mruby.h>
#include <mruby/compile.h>
#include <mruby/dump.h>
#include <mruby/proc.h>
#include <mruby/internal.h>
#include <mruby/version.h>

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char *
openblink_strdup(const char *s)
{
  size_t n = strlen(s) + 1;
  char *p = (char *)malloc(n);
  if (p) memcpy(p, s, n);
  return p;
}

/*
 * Compile Ruby source into RITE bytecode (.mrb) held in memory.
 *
 * Returns 0 on success and sets *out_buf (malloc'd, *out_len bytes).
 * Returns non-zero on failure and sets *err_msg (malloc'd C string) to a
 * message formatted as "file:line:col: message" for syntax errors.
 */
int
openblink_mrbc_compile(const char *src, size_t src_len, const char *filename,
                       uint8_t **out_buf, size_t *out_len, char **err_msg)
{
  *out_buf = NULL;
  *out_len = 0;
  *err_msg = NULL;

  mrb_state *mrb = mrb_open();
  if (mrb == NULL) {
    *err_msg = openblink_strdup("failed to initialize mruby state");
    return 1;
  }

  const char *fname = (filename && filename[0]) ? filename : "(input)";

  mrb_ccontext *c = mrb_ccontext_new(mrb);
  if (c == NULL) {
    mrb_close(mrb);
    *err_msg = openblink_strdup("failed to allocate compiler context");
    return 1;
  }
  c->capture_errors = TRUE;
  c->no_exec = TRUE;
  mrb_ccontext_filename(mrb, c, fname);

  struct mrb_parser_state *p = mrb_parse_nstring(mrb, src, src_len, c);
  if (p == NULL) {
    mrb_ccontext_free(mrb, c);
    mrb_close(mrb);
    *err_msg = openblink_strdup("failed to allocate parser");
    return 1;
  }

  if (p->nerr > 0) {
    struct mrb_parser_message *e = &p->error_buffer[0];
    const char *msg = e->message ? e->message : "syntax error";
    int needed = snprintf(NULL, 0, "%s:%d:%d: %s", fname, (int)e->lineno, e->column, msg);
    if (needed < 0) needed = 0;
    char *buf = (char *)malloc((size_t)needed + 1);
    if (buf) {
      snprintf(buf, (size_t)needed + 1, "%s:%d:%d: %s", fname, (int)e->lineno, e->column, msg);
    }
    *err_msg = buf ? buf : openblink_strdup("syntax error");
    mrb_parser_free(p);
    mrb_ccontext_free(mrb, c);
    mrb_close(mrb);
    return 1;
  }

  struct RProc *proc = mrb_generate_code(mrb, p);
  mrb_parser_free(p);
  if (proc == NULL) {
    mrb_ccontext_free(mrb, c);
    mrb_close(mrb);
    *err_msg = openblink_strdup("failed to generate bytecode");
    return 1;
  }

  uint8_t *bin = NULL;
  size_t bin_size = 0;
  int dump_result = mrb_dump_irep(mrb, proc->body.irep, 0, &bin, &bin_size);
  if (dump_result != MRB_DUMP_OK || bin == NULL) {
    if (bin) mrb_free(mrb, bin);
    mrb_ccontext_free(mrb, c);
    mrb_close(mrb);
    *err_msg = openblink_strdup("failed to dump bytecode");
    return 1;
  }

  uint8_t *out = (uint8_t *)malloc(bin_size > 0 ? bin_size : 1);
  if (out == NULL) {
    mrb_free(mrb, bin);
    mrb_ccontext_free(mrb, c);
    mrb_close(mrb);
    *err_msg = openblink_strdup("out of memory");
    return 1;
  }
  memcpy(out, bin, bin_size);

  mrb_free(mrb, bin);
  mrb_ccontext_free(mrb, c);
  mrb_close(mrb);

  *out_buf = out;
  *out_len = bin_size;
  return 0;
}

void
openblink_mrbc_free(void *ptr)
{
  free(ptr);
}

const char *
openblink_mrbc_version(void)
{
  return MRUBY_RUBY_ENGINE " " MRUBY_VERSION; /* e.g. "mruby 3.4.0" */
}
