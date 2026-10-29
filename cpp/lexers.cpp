// 使用する Lexilla レクサーだけを静的リンクするための薄いカタログ
#include <cstring>
#include <vector>
#include <initializer_list>

#include "ILexer.h"
#include "LexerModule.h"
#include "CatalogueModules.h"

using namespace Lexilla;

extern const LexerModule lmJSON;
extern const LexerModule lmCPP;
extern const LexerModule lmPython;
extern const LexerModule lmProps;
extern const LexerModule lmBatch;
extern const LexerModule lmSQL;
extern const LexerModule lmHTML;
extern const LexerModule lmXML;
extern const LexerModule lmMarkdown;
extern const LexerModule lmYAML;
extern const LexerModule lmPowerShell;
extern const LexerModule lmRust;
extern const LexerModule lmCss;
extern const LexerModule lmBash;
extern const LexerModule lmDiff;
extern const LexerModule lmNull;

static CatalogueModules catalogue;
static bool initialised = false;

extern "C" void *sk_create_lexer(const char *name) {
	if (!initialised) {
		catalogue.AddLexerModules({
			&lmJSON, &lmCPP, &lmPython, &lmProps, &lmBatch, &lmSQL, &lmHTML, &lmXML,
			&lmMarkdown, &lmYAML, &lmPowerShell, &lmRust, &lmCss, &lmBash, &lmDiff, &lmNull,
		});
		initialised = true;
	}
	for (size_t i = 0; i < catalogue.Count(); i++) {
		if (strcmp(catalogue.Name(i), name) == 0) {
			return catalogue.Create(i);
		}
	}
	return nullptr;
}
