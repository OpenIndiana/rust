use crate::utils::{is_try, match_qpath, match_trait_method, paths, span_lint};
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:** Checks for unused written/read amount.
    ///
    /// **Why is this bad?** `io::Write::write(_vectored)` and
    /// `io::Read::read(_vectored)` are not guaranteed to
    /// process the entire buffer. They return how many bytes were processed, which
    /// might be smaller
    /// than a given buffer's length. If you don't need to deal with
    /// partial-write/read, use
    /// `write_all`/`read_exact` instead.
    ///
    /// **Known problems:** Detects only common patterns.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// use std::io;
    /// fn foo<W: io::Write>(w: &mut W) -> io::Result<()> {
    ///     // must be `w.write_all(b"foo")?;`
    ///     w.write(b"foo")?;
    ///     Ok(())
    /// }
    /// ```
    pub UNUSED_IO_AMOUNT,
    correctness,
    "unused written/read amount"
}

declare_lint_pass!(UnusedIoAmount => [UNUSED_IO_AMOUNT]);

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for UnusedIoAmount {
    fn check_stmt(&mut self, cx: &LateContext<'_, '_>, s: &hir::Stmt<'_>) {
        let expr = match s.kind {
            hir::StmtKind::Semi(ref expr) | hir::StmtKind::Expr(ref expr) => &**expr,
            _ => return,
        };

        match expr.kind {
            hir::ExprKind::Match(ref res, _, _) if is_try(expr).is_some() => {
                if let hir::ExprKind::Call(ref func, ref args) = res.kind {
                    if let hir::ExprKind::Path(ref path) = func.kind {
                        if match_qpath(path, &paths::TRY_INTO_RESULT) && args.len() == 1 {
                            check_method_call(cx, &args[0], expr);
                        }
                    }
                } else {
                    check_method_call(cx, res, expr);
                }
            },

            hir::ExprKind::MethodCall(ref path, _, ref args) => match &*path.ident.as_str() {
                "expect" | "unwrap" | "unwrap_or" | "unwrap_or_else" => {
                    check_method_call(cx, &args[0], expr);
                },
                _ => (),
            },

            _ => (),
        }
    }
}

fn check_method_call(cx: &LateContext<'_, '_>, call: &hir::Expr<'_>, expr: &hir::Expr<'_>) {
    if let hir::ExprKind::MethodCall(ref path, _, _) = call.kind {
        let symbol = &*path.ident.as_str();
        let read_trait = match_trait_method(cx, call, &paths::IO_READ);
        let write_trait = match_trait_method(cx, call, &paths::IO_WRITE);

        match (read_trait, write_trait, symbol) {
            (true, _, "read") => span_lint(
                cx,
                UNUSED_IO_AMOUNT,
                expr.span,
                "read amount is not handled. Use `Read::read_exact` instead",
            ),
            (true, _, "read_vectored") => span_lint(cx, UNUSED_IO_AMOUNT, expr.span, "read amount is not handled"),
            (_, true, "write") => span_lint(
                cx,
                UNUSED_IO_AMOUNT,
                expr.span,
                "written amount is not handled. Use `Write::write_all` instead",
            ),
            (_, true, "write_vectored") => span_lint(cx, UNUSED_IO_AMOUNT, expr.span, "written amount is not handled"),
            _ => (),
        }
    }
}
