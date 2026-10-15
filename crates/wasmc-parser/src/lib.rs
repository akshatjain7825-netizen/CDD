use wasmc_ast::*;
use wasmc_diagnostics::{Diagnostic, DiagnosticSink, SourceSpan};
use wasmc_lexer::{SymbolId, Token, TokenKind};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    cursor: usize,
    sink: &'a mut DiagnosticSink,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, sink: &'a mut DiagnosticSink) -> Self {
        Self {
            tokens,
            cursor: 0,
            sink,
        }
    }

    fn peek(&self) -> &Token {
        if self.cursor < self.tokens.len() {
            &self.tokens[self.cursor]
        } else {
            self.tokens.last().unwrap()
        }
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_span(&self) -> SourceSpan {
        self.peek().span
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.cursor < self.tokens.len() {
            self.cursor += 1;
        }
        tok
    }

    fn check(&self, expected: &TokenKind) -> bool {
        match (self.peek_kind(), expected) {
            (TokenKind::Identifier(_), TokenKind::Identifier(_)) => true,
            (TokenKind::StringLiteral(_), TokenKind::StringLiteral(_)) => true,
            (TokenKind::Integer(_), TokenKind::Integer(_)) => true,
            (TokenKind::Float(_), TokenKind::Float(_)) => true,
            (a, b) => a == b,
        }
    }

    fn match_token(&mut self, expected: &TokenKind) -> Option<Token> {
        if self.check(expected) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn expect(&mut self, expected: &TokenKind, msg: &str) -> Result<Token, SourceSpan> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            let span = self.peek_span();
            self.sink.push(Diagnostic::error(
                "E2001",
                format!("Expected {}, found {:?}", msg, self.peek_kind()),
                span,
            ));
            Err(span)
        }
    }

    fn synchronize(&mut self) {
        if self.peek_kind() == &TokenKind::Eof {
            return;
        }
        self.advance();
        while self.peek_kind() != &TokenKind::Eof {
            if self.peek_kind() == &TokenKind::Semicolon {
                self.advance();
                return;
            }
            match self.peek_kind() {
                TokenKind::Fn
                | TokenKind::Struct
                | TokenKind::Global
                | TokenKind::Import
                | TokenKind::Export => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub fn parse_module(&mut self, pool: &mut AstPool) {
        while self.peek_kind() != &TokenKind::Eof {
            match self.parse_top_level_decl(pool) {
                Ok(idx) => pool.top_level_decls.push(idx),
                Err(_) => {
                    self.synchronize();
                }
            }
        }
    }

    fn parse_top_level_decl(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.peek_span();
        match self.peek_kind() {
            TokenKind::Import => self.parse_import_decl(pool),
            TokenKind::Export => {
                self.advance();
                let inner = self.parse_top_level_decl(pool)?;
                let end_span = pool.span(inner);
                Ok(pool.alloc(
                    AstNode::ExportDecl { decl: inner },
                    start_span.merge(end_span),
                ))
            }
            TokenKind::Struct => self.parse_struct_decl(pool),
            TokenKind::Global => self.parse_global_decl(pool),
            TokenKind::Fn => self.parse_function_decl(pool),
            _ => {
                let span = self.peek_span();
                self.sink.push(Diagnostic::error(
                    "E2001",
                    format!(
                        "Expected top-level declaration (fn, struct, global, import, export), found {:?}",
                        self.peek_kind()
                    ),
                    span,
                ));
                Err(span)
            }
        }
    }

    fn parse_import_decl(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::Import, "'import'")?.span;
        let mod_tok = self.expect(&TokenKind::StringLiteral(SymbolId(0)), "module string")?;
        let field_tok = self.expect(&TokenKind::StringLiteral(SymbolId(0)), "field string")?;
        self.expect(&TokenKind::Fn, "'fn'")?;

        let name_tok = self.expect(&TokenKind::Identifier(SymbolId(0)), "function name")?;
        self.expect(&TokenKind::OpenParen, "'('")?;

        let mut params = Vec::new();
        if self.peek_kind() != &TokenKind::CloseParen {
            loop {
                let p_name = self.expect(&TokenKind::Identifier(SymbolId(0)), "parameter name")?;
                self.expect(&TokenKind::Colon, "':'")?;
                let p_type = self.parse_type(pool)?;
                if let TokenKind::Identifier(sym) = p_name.kind {
                    params.push(Param {
                        name: sym,
                        type_node: p_type,
                    });
                }
                if self.match_token(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(&TokenKind::CloseParen, "')'")?;

        let mut return_type = None;
        if self.match_token(&TokenKind::Arrow).is_some() {
            return_type = Some(self.parse_type(pool)?);
        }
        let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;

        let mod_sym = match mod_tok.kind {
            TokenKind::StringLiteral(s) => s,
            _ => unreachable!(),
        };
        let field_sym = match field_tok.kind {
            TokenKind::StringLiteral(s) => s,
            _ => unreachable!(),
        };
        let fn_sym = match name_tok.kind {
            TokenKind::Identifier(s) => s,
            _ => unreachable!(),
        };

        Ok(pool.alloc(
            AstNode::ImportDecl {
                module: mod_sym,
                field: field_sym,
                func_name: fn_sym,
                params,
                return_type,
            },
            start_span.merge(end_span),
        ))
    }

    fn parse_struct_decl(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::Struct, "'struct'")?.span;
        let name_tok = self.expect(&TokenKind::Identifier(SymbolId(0)), "struct name")?;
        self.expect(&TokenKind::OpenBrace, "'{'")?;

        let mut fields = Vec::new();
        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            let f_name = self.expect(&TokenKind::Identifier(SymbolId(0)), "field name")?;
            self.expect(&TokenKind::Colon, "':'")?;
            let f_type = self.parse_type(pool)?;
            if let TokenKind::Identifier(sym) = f_name.kind {
                fields.push(StructField {
                    name: sym,
                    type_node: f_type,
                });
            }
            if self.match_token(&TokenKind::Comma).is_none() {
                break;
            }
        }
        let end_span = self.expect(&TokenKind::CloseBrace, "'}'")?.span;

        let name_sym = match name_tok.kind {
            TokenKind::Identifier(s) => s,
            _ => unreachable!(),
        };

        Ok(pool.alloc(
            AstNode::StructDecl {
                name: name_sym,
                fields,
            },
            start_span.merge(end_span),
        ))
    }

    fn parse_global_decl(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::Global, "'global'")?.span;
        let mutable = self.match_token(&TokenKind::Mut).is_some();
        let name_tok = self.expect(&TokenKind::Identifier(SymbolId(0)), "global identifier")?;
        self.expect(&TokenKind::Colon, "':'")?;
        let type_node = self.parse_type(pool)?;
        self.expect(&TokenKind::Eq, "'='")?;
        let init_expr = self.parse_expr(pool)?;
        let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;

        let name_sym = match name_tok.kind {
            TokenKind::Identifier(s) => s,
            _ => unreachable!(),
        };

        Ok(pool.alloc(
            AstNode::GlobalDecl {
                name: name_sym,
                mutable,
                type_node,
                init_expr,
            },
            start_span.merge(end_span),
        ))
    }

    fn parse_function_decl(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::Fn, "'fn'")?.span;
        let name_tok = self.expect(&TokenKind::Identifier(SymbolId(0)), "function name")?;
        self.expect(&TokenKind::OpenParen, "'('")?;

        let mut params = Vec::new();
        if self.peek_kind() != &TokenKind::CloseParen {
            loop {
                let p_name = self.expect(&TokenKind::Identifier(SymbolId(0)), "parameter name")?;
                self.expect(&TokenKind::Colon, "':'")?;
                let p_type = self.parse_type(pool)?;
                if let TokenKind::Identifier(sym) = p_name.kind {
                    params.push(Param {
                        name: sym,
                        type_node: p_type,
                    });
                }
                if self.match_token(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(&TokenKind::CloseParen, "')'")?;

        let mut return_type = None;
        if self.match_token(&TokenKind::Arrow).is_some() {
            return_type = Some(self.parse_type(pool)?);
        }

        let body = self.parse_block_stmt(pool)?;
        let end_span = pool.span(body);

        let name_sym = match name_tok.kind {
            TokenKind::Identifier(s) => s,
            _ => unreachable!(),
        };

        Ok(pool.alloc(
            AstNode::FunctionDecl {
                name: name_sym,
                params,
                return_type,
                body,
            },
            start_span.merge(end_span),
        ))
    }

    pub fn parse_type(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let span = self.peek_span();
        match self.peek_kind() {
            TokenKind::I32 => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::I32), span))
            }
            TokenKind::I64 => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::I64), span))
            }
            TokenKind::U32 => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::U32), span))
            }
            TokenKind::U64 => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::U64), span))
            }
            TokenKind::F32 => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::F32), span))
            }
            TokenKind::F64 => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::F64), span))
            }
            TokenKind::Bool => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::Bool), span))
            }
            TokenKind::Char => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::Char), span))
            }
            TokenKind::Void => {
                self.advance();
                Ok(pool.alloc(AstNode::TypePrimitive(PrimType::Void), span))
            }
            TokenKind::Star => {
                self.advance();
                let inner = self.parse_type(pool)?;
                let end_span = pool.span(inner);
                Ok(pool.alloc(AstNode::TypePointer(inner), span.merge(end_span)))
            }
            TokenKind::OpenBracket => {
                self.advance();
                let inner = self.parse_type(pool)?;
                self.expect(&TokenKind::Semicolon, "';'")?;
                let len_tok = self.expect(&TokenKind::Integer(0), "array length integer")?;
                let end_span = self.expect(&TokenKind::CloseBracket, "']'")?.span;
                let len = match len_tok.kind {
                    TokenKind::Integer(i) => i as u32,
                    _ => 0,
                };
                Ok(pool.alloc(AstNode::TypeArray(inner, len), span.merge(end_span)))
            }
            TokenKind::Fn => {
                self.advance();
                self.expect(&TokenKind::OpenParen, "'('")?;
                let mut params = Vec::new();
                if self.peek_kind() != &TokenKind::CloseParen {
                    loop {
                        params.push(self.parse_type(pool)?);
                        if self.match_token(&TokenKind::Comma).is_none() {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::CloseParen, "')'")?;
                let mut ret = None;
                if self.match_token(&TokenKind::Arrow).is_some() {
                    ret = Some(self.parse_type(pool)?);
                }
                let end_span = ret.map(|r| pool.span(r)).unwrap_or(span);
                Ok(pool.alloc(
                    AstNode::TypeFuncPtr {
                        params,
                        return_type: ret,
                    },
                    span.merge(end_span),
                ))
            }
            TokenKind::Identifier(sym) => {
                let s = *sym;
                self.advance();
                Ok(pool.alloc(AstNode::TypeStructRef(s), span))
            }
            _ => {
                self.sink.push(Diagnostic::error(
                    "E2001",
                    format!("Expected type signature, found {:?}", self.peek_kind()),
                    span,
                ));
                Err(span)
            }
        }
    }

    pub fn parse_stmt(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let span = self.peek_span();
        match self.peek_kind() {
            TokenKind::OpenBrace => self.parse_block_stmt(pool),
            TokenKind::Let => self.parse_let_stmt(pool),
            TokenKind::If => self.parse_if_stmt(pool),
            TokenKind::While => self.parse_while_stmt(pool),
            TokenKind::Return => self.parse_return_stmt(pool),
            TokenKind::Break => {
                self.advance();
                let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;
                Ok(pool.alloc(AstNode::BreakStmt, span.merge(end_span)))
            }
            TokenKind::Continue => {
                self.advance();
                let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;
                Ok(pool.alloc(AstNode::ContinueStmt, span.merge(end_span)))
            }
            TokenKind::Try => self.parse_try_catch_stmt(pool),
            TokenKind::Throw => {
                self.advance();
                let expr = self.parse_expr(pool)?;
                let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;
                Ok(pool.alloc(AstNode::ThrowStmt(expr), span.merge(end_span)))
            }
            _ => self.parse_expr_or_assign_stmt(pool),
        }
    }

    fn parse_block_stmt(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::OpenBrace, "'{'")?.span;
        let mut stmts = Vec::new();
        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.parse_stmt(pool) {
                Ok(idx) => stmts.push(idx),
                Err(_) => {
                    self.synchronize();
                    stmts.push(pool.alloc(AstNode::ErrorStmt, self.peek_span()));
                }
            }
        }
        let end_span = self.expect(&TokenKind::CloseBrace, "'}'")?.span;
        Ok(pool.alloc(AstNode::BlockStmt(stmts), start_span.merge(end_span)))
    }

    fn parse_let_stmt(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::Let, "'let'")?.span;
        let mutable = self.match_token(&TokenKind::Mut).is_some();
        let name_tok = self.expect(&TokenKind::Identifier(SymbolId(0)), "variable name")?;
        self.expect(&TokenKind::Colon, "':'")?;
        let type_node = self.parse_type(pool)?;

        let mut init = None;
        if self.match_token(&TokenKind::Eq).is_some() {
            init = Some(self.parse_expr(pool)?);
        }
        let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;

        let sym = match name_tok.kind {
            TokenKind::Identifier(s) => s,
            _ => unreachable!(),
        };

        Ok(pool.alloc(
            AstNode::LetStmt {
                name: sym,
                mutable,
                type_node,
                init,
            },
            start_span.merge(end_span),
        ))
    }

    fn parse_if_stmt(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::If, "'if'")?.span;
        self.expect(&TokenKind::OpenParen, "'('")?;
        let cond = self.parse_expr(pool)?;
        self.expect(&TokenKind::CloseParen, "')'")?;
        let then_block = self.parse_block_stmt(pool)?;

        let mut else_stmt = None;
        if self.match_token(&TokenKind::Else).is_some() {
            if self.peek_kind() == &TokenKind::If {
                else_stmt = Some(self.parse_if_stmt(pool)?);
            } else {
                else_stmt = Some(self.parse_block_stmt(pool)?);
            }
        }

        let end_span = else_stmt
            .map(|e| pool.span(e))
            .unwrap_or_else(|| pool.span(then_block));
        Ok(pool.alloc(
            AstNode::IfStmt {
                cond,
                then_block,
                else_stmt,
            },
            start_span.merge(end_span),
        ))
    }

    fn parse_while_stmt(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::While, "'while'")?.span;
        self.expect(&TokenKind::OpenParen, "'('")?;
        let cond = self.parse_expr(pool)?;
        self.expect(&TokenKind::CloseParen, "')'")?;
        let body = self.parse_block_stmt(pool)?;
        let end_span = pool.span(body);
        Ok(pool.alloc(AstNode::WhileStmt { cond, body }, start_span.merge(end_span)))
    }

    fn parse_return_stmt(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::Return, "'return'")?.span;
        let is_tail = self.match_token(&TokenKind::Tail).is_some();

        let mut val = None;
        if self.peek_kind() != &TokenKind::Semicolon {
            val = Some(self.parse_expr(pool)?);
        }
        let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;
        Ok(pool.alloc(
            AstNode::ReturnStmt {
                is_tail,
                value: val,
            },
            start_span.merge(end_span),
        ))
    }

    fn parse_try_catch_stmt(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.expect(&TokenKind::Try, "'try'")?.span;
        let try_block = self.parse_block_stmt(pool)?;
        self.expect(&TokenKind::Catch, "'catch'")?;
        self.expect(&TokenKind::OpenParen, "'('")?;
        let err_name = self.expect(&TokenKind::Identifier(SymbolId(0)), "error identifier")?;
        self.expect(&TokenKind::Colon, "':'")?;
        let err_type = self.parse_type(pool)?;
        self.expect(&TokenKind::CloseParen, "')'")?;
        let catch_block = self.parse_block_stmt(pool)?;
        let end_span = pool.span(catch_block);

        let sym = match err_name.kind {
            TokenKind::Identifier(s) => s,
            _ => unreachable!(),
        };

        Ok(pool.alloc(
            AstNode::TryCatchStmt {
                try_block,
                err_name: sym,
                err_type,
                catch_block,
            },
            start_span.merge(end_span),
        ))
    }

    fn parse_expr_or_assign_stmt(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let start_span = self.peek_span();
        let expr = self.parse_expr(pool)?;

        if self.match_token(&TokenKind::Eq).is_some() {
            let rhs = self.parse_expr(pool)?;
            let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;
            Ok(pool.alloc(
                AstNode::AssignStmt { lhs: expr, rhs },
                start_span.merge(end_span),
            ))
        } else {
            let end_span = self.expect(&TokenKind::Semicolon, "';'")?.span;
            Ok(pool.alloc(AstNode::ExprStmt(expr), start_span.merge(end_span)))
        }
    }

    pub fn parse_expr(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        self.parse_expr_bp(0, pool)
    }

    fn parse_expr_bp(&mut self, min_bp: u8, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let mut lhs = self.parse_unary_or_primary(pool)?;

        loop {
            // Postfix operators: (), [], .
            if self.peek_kind() == &TokenKind::OpenParen && 20 >= min_bp {
                self.advance();
                let mut args = Vec::new();
                if self.peek_kind() != &TokenKind::CloseParen {
                    loop {
                        args.push(self.parse_expr(pool)?);
                        if self.match_token(&TokenKind::Comma).is_none() {
                            break;
                        }
                    }
                }
                let end_span = self.expect(&TokenKind::CloseParen, "')'")?.span;
                let span = pool.span(lhs).merge(end_span);
                lhs = pool.alloc(AstNode::CallExpr { callee: lhs, args }, span);
                continue;
            }

            if self.peek_kind() == &TokenKind::OpenBracket && 20 >= min_bp {
                self.advance();
                let idx = self.parse_expr(pool)?;
                let end_span = self.expect(&TokenKind::CloseBracket, "']'")?.span;
                let span = pool.span(lhs).merge(end_span);
                lhs = pool.alloc(AstNode::IndexExpr { base: lhs, index: idx }, span);
                continue;
            }

            if self.peek_kind() == &TokenKind::Dot && 20 >= min_bp {
                self.advance();
                let member_tok = self.expect(&TokenKind::Identifier(SymbolId(0)), "member name")?;
                let mem_sym = match member_tok.kind {
                    TokenKind::Identifier(s) => s,
                    _ => unreachable!(),
                };
                let span = pool.span(lhs).merge(member_tok.span);
                lhs = pool.alloc(AstNode::MemberExpr { base: lhs, member: mem_sym }, span);
                continue;
            }

            // Infix binary operators
            let (op, l_bp, r_bp) = match self.peek_kind() {
                TokenKind::PipePipe => (BinaryOp::LogicOr, 1, 2),
                TokenKind::AmpAmp => (BinaryOp::LogicAnd, 3, 4),
                TokenKind::Pipe => (BinaryOp::BitOr, 5, 6),
                TokenKind::Caret => (BinaryOp::BitXor, 5, 6),
                TokenKind::Ampersand => (BinaryOp::BitAnd, 5, 6),
                TokenKind::EqEq => (BinaryOp::Eq, 7, 8),
                TokenKind::BangEq => (BinaryOp::Ne, 7, 8),
                TokenKind::Lt => (BinaryOp::Lt, 9, 10),
                TokenKind::LtEq => (BinaryOp::Le, 9, 10),
                TokenKind::Gt => (BinaryOp::Gt, 9, 10),
                TokenKind::GtEq => (BinaryOp::Ge, 9, 10),
                TokenKind::Shl => (BinaryOp::Shl, 11, 12),
                TokenKind::Shr => (BinaryOp::Shr, 11, 12),
                TokenKind::Plus => (BinaryOp::Add, 13, 14),
                TokenKind::Minus => (BinaryOp::Sub, 13, 14),
                TokenKind::Star => (BinaryOp::Mul, 15, 16),
                TokenKind::Slash => (BinaryOp::Div, 15, 16),
                TokenKind::Percent => (BinaryOp::Rem, 15, 16),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.advance();
            let rhs = self.parse_expr_bp(r_bp, pool)?;
            let span = pool.span(lhs).merge(pool.span(rhs));
            lhs = pool.alloc(AstNode::BinaryExpr { op, lhs, rhs }, span);
        }

        Ok(lhs)
    }

    fn parse_unary_or_primary(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let span = self.peek_span();
        match self.peek_kind() {
            TokenKind::Minus => {
                self.advance();
                let opnd = self.parse_expr_bp(18, pool)?;
                let s = span.merge(pool.span(opnd));
                Ok(pool.alloc(AstNode::UnaryExpr { op: UnaryOp::Neg, operand: opnd }, s))
            }
            TokenKind::Bang => {
                self.advance();
                let opnd = self.parse_expr_bp(18, pool)?;
                let s = span.merge(pool.span(opnd));
                Ok(pool.alloc(AstNode::UnaryExpr { op: UnaryOp::Not, operand: opnd }, s))
            }
            TokenKind::Tilde => {
                self.advance();
                let opnd = self.parse_expr_bp(18, pool)?;
                let s = span.merge(pool.span(opnd));
                Ok(pool.alloc(AstNode::UnaryExpr { op: UnaryOp::BitNot, operand: opnd }, s))
            }
            TokenKind::Ampersand => {
                self.advance();
                let opnd = self.parse_expr_bp(18, pool)?;
                let s = span.merge(pool.span(opnd));
                Ok(pool.alloc(AstNode::UnaryExpr { op: UnaryOp::AddrOf, operand: opnd }, s))
            }
            TokenKind::Star => {
                self.advance();
                let opnd = self.parse_expr_bp(18, pool)?;
                let s = span.merge(pool.span(opnd));
                Ok(pool.alloc(AstNode::UnaryExpr { op: UnaryOp::Deref, operand: opnd }, s))
            }
            _ => self.parse_primary(pool),
        }
    }

    fn parse_primary(&mut self, pool: &mut AstPool) -> Result<NodeIndex, SourceSpan> {
        let span = self.peek_span();
        match self.peek_kind() {
            TokenKind::Integer(n) => {
                let val = *n;
                self.advance();
                Ok(pool.alloc(AstNode::LiteralExpr(LiteralValue::Integer(val)), span))
            }
            TokenKind::Float(f) => {
                let val = *f;
                self.advance();
                Ok(pool.alloc(AstNode::LiteralExpr(LiteralValue::Float(val)), span))
            }
            TokenKind::BoolLiteral(b) => {
                let val = *b;
                self.advance();
                Ok(pool.alloc(AstNode::LiteralExpr(LiteralValue::Bool(val)), span))
            }
            TokenKind::CharLiteral(c) => {
                let val = *c;
                self.advance();
                Ok(pool.alloc(AstNode::LiteralExpr(LiteralValue::Char(val)), span))
            }
            TokenKind::StringLiteral(s) => {
                let val = *s;
                self.advance();
                Ok(pool.alloc(AstNode::LiteralExpr(LiteralValue::String(val)), span))
            }
            TokenKind::Identifier(sym) => {
                let val = *sym;
                self.advance();
                Ok(pool.alloc(AstNode::VariableExpr(val), span))
            }
            TokenKind::OpenParen => {
                self.advance();
                let expr = self.parse_expr(pool)?;
                self.expect(&TokenKind::CloseParen, "')'")?;
                Ok(expr)
            }
            _ => {
                self.sink.push(Diagnostic::error(
                    "E2001",
                    format!("Unexpected token in expression: {:?}", self.peek_kind()),
                    span,
                ));
                Err(span)
            }
        }
    }
}
