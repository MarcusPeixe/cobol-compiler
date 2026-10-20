#import "@preview/lambda-notes:0.1.0": *

#show: lambda-notes.with(
  title: [
    Relatório #sym.dot.c Lexer
  ],
  author: [
    Diego Pedroso de Oliveira

    Marcus Vinícius Araújo Peixoto

    Rafael Moura de Almeida

  ],
  date: [
    Outubro de 2026 #sym.dot.c
  ],
  subject: "Implementação e testes de um analisador léxico COBOL",
  keywords: ("COBOL", "lexer", "Rust", "compiladores"),
  color: rgb("#333"),
  show-outline: true,
  outline-title: [Sumário],
  chapter-label: [Seção],
)

#set par(justify: true)

#callout(title: "Antes da entrega", color: orange)[
  Este arquivo é um modelo de relatório. Substitua todos os campos marcados como *PLACEHOLDER*, especialmente os nomes dos integrantes, a identificação da disciplina e os blocos reservados aos resultados reais. Os comandos abaixo devem ser executados a partir da raiz do repositório.
]

= Introdução

COBOL (*Common Business-Oriented Language*) é uma linguagem de programação criada para aplicações de processamento de dados de negócio. Ela continua presente em sistemas corporativos, especialmente em ambientes legados, por sua longa vida útil, grande quantidade de programas existentes e foco em operações com dados e registros.

A análise léxica de COBOL apresenta desafios que não são comuns em linguagens com formato livre. No formato fixo, as colunas têm significado: as colunas 1--6 podem conter números de sequência, a coluna 7 indica comentário, continuação ou linha de _debugging_, as colunas 8--72 contêm o código e as colunas seguintes são ignoradas. Literais podem continuar na linha seguinte. Além disso, COBOL não diferencia maiúsculas de minúsculas, e o ponto pode separar sentenças ou fazer parte de um literal numérico.

Este trabalho implementa a primeira etapa de um compilador COBOL: a conversão do texto-fonte em uma sequência de tokens, com diagnóstico de erros léxicos.

= Implementação

== Estratégia e ferramentas

O projeto foi desenvolvido em _Rust 2024 edition_. O código foi estruturado como uma biblioteca (`cobol_compiler`) acompanhada por uma interface de linha de comando. A separação permite que os testes chamem diretamente as funções da biblioteca, enquanto o comando `lex` oferece uma forma de uso independente.

O fluxo de processamento é:

1. leitura do arquivo no formato fixo;
2. expansão de diretivas `COPY` e leitura dos copybooks;
3. análise léxica e geração de tokens e diagnósticos.

A leitura fixa remove as áreas de sequência e de identificação do programa, descarta linhas de comentário e, por padrão, linhas de _debugging_. O lexer reconhece palavras-chave, identificadores, literais, símbolos e strings de picture. Palavras e identificadores são tratados de modo _case insensitive_. Os diagnósticos incluem localização no arquivo original; erros são apresentados ao usuário e fazem o comando terminar com código de falha.

As principais ferramentas e dependências utilizadas são:

#align(center, table(
  columns: 2,
  [*Ferramenta*], [*Uso*],
  [*Rust e Cargo*], [Implementação, compilação e testes],
  [*clap*], [Argumentos do comando],
  [*thiserror*], [Erros tipados],
  [*anyhow*], [Propagação de erros na interface de linha de comando],
  [*miette*], [Apresentação formatada dos diagnósticos],
  [*Nix*], [Fornece o ambiente de desenvolvimento Rust],
))

Não foi utilizada uma ferramenta geradora de lexer como Flex: o analisador foi implementado diretamente em Rust. Essa decisão foi tomada devido à complexidade léxica do COBOL, que atribui significado às diferentes colunas do código-fonte (_fixed source format_).

== Uso de ferramentas de inteligência artificial

Foi utilizada assistência de IA (Github Copilot Pro) integrada ao ambiente de desenvolvimento para apoiar a implementação e revisão do código, sugerir e auxiliar na elaboração de testes, participar do _brainstorming_ de decisões técnicas, e preparar e manter a documentação. As sugestões foram verificadas pela equipe por meio da leitura do código e da execução dos testes. Os modelos efetivamente utilizados foram GPT-6 Luna e Claude Sonnet 5.5.

== Repositório

O código-fonte está disponível no _branch_ `lexer` do repositório no GitHub:

#link("https://github.com/MarcusPeixe/cobol-compiler/tree/lexer")[github.com/MarcusPeixe/cobol-compiler/tree/lexer].

= Resultados dos testes

== Teste de regressão automatizado

Os sete programas fornecidos foram adicionados como casos de regressão. O teste de integração executa o leitor de formato fixo, o pré-processador e o lexer para cada exemplo, formata os tokens e compara o resultado com o arquivo de referência correspondente em `tests/lexer-goldens/`. Os avisos são registrados numa seção de diagnósticos do golden; diagnósticos de erro fazem o teste falhar.

Execute a suíte completa na raiz do repositório:

```sh
cargo test
```

Para executar somente o teste dos exemplos:

```sh
cargo test --test lexer_golden_tests
```

```
$ cargo test
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running unittests src/lib.rs (target/debug/deps/cobol_compiler-545d32a9d477cf56)

running 101 tests
test diagnostics::tests::severity_ordering_and_letters ... ok
test lexer::scanner::tests::doubled_quotes_are_escapes ... ok
...
test source::tests::synthetic_chars_reuse_origin ... ok
test preprocess::tests::fs_loader_finds_files_with_extensions_and_case ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/lexer_golden_tests.rs (target/debug/deps/lexer_golden_tests-b985524a09d9febf)

running 1 test
test professor_examples_match_lexer_goldens ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

== Fluxo de tokens de cada programa

Para gerar a saída diretamente pelo CLI, execute um comando por arquivo. Por exemplo:

```sh
cargo run -- lex cobol-examples/prog001_divisoes.cob
```

Para guardar separadamente a saída de tokens e os diagnósticos:

```sh
cargo run -- lex cobol-examples/prog001_divisoes.cob \
  > /tmp/prog001.tokens 2> /tmp/prog001.diagnostics
```

Repita para os outros seis nomes listados abaixo, alterando o nome do arquivo e os nomes de saída. O `stdout` contém os tokens; avisos e erros são escritos no `stderr`. Os fluxos completos podem ser extensos, por isso cada bloco seguinte está reservado para a saída produzida na execução.

=== `prog001_divisoes.cob`

Comando:
`cargo run -- lex cobol-examples/prog001_divisoes.cob`

```
PLACEHOLDER: cole aqui a saída completa de tokens deste programa.
```

=== `prog002_operacoes.cob`

Comando:
`cargo run -- lex cobol-examples/prog002_operacoes.cob`

```
PLACEHOLDER: cole aqui a saída completa de tokens deste programa.
```

=== `prog003_calculadora.cob`

Comando:
`cargo run -- lex cobol-examples/prog003_calculadora.cob`

```
PLACEHOLDER: cole aqui a saída completa de tokens deste programa.
```

=== `prog004_folha_pgto.cob`

Comando:
`cargo run -- lex cobol-examples/prog004_folha_pgto.cob`

```
PLACEHOLDER: cole aqui a saída completa de tokens deste programa.
```

=== `prog005_strings.cob`

Comando:
`cargo run -- lex cobol-examples/prog005_strings.cob`

```
PLACEHOLDER: cole aqui a saída completa de tokens deste programa.
```

=== `prog006_files.cob`

Comando:
`cargo run -- lex cobol-examples/prog006_files.cob`

```
PLACEHOLDER: cole aqui a saída completa de tokens deste programa.
```

=== `prog007_loja.cob`

Comando:
`cargo run -- lex cobol-examples/prog007_loja.cob`

```
PLACEHOLDER: cole aqui a saída completa de tokens deste programa.
```

PLACEHOLDER: para cada arquivo, inclua também os avisos correspondentes capturados em `stderr`, se houver. Não use a apresentação visual do `miette` como golden; transcreva ou simplifique a mensagem mantendo severidade, código e localização.

= Resultados de erros

== Procedimento reproduzível

Para demonstrar a recuperação e a emissão de diagnósticos, crie o arquivo `documents/lexer-errors.cob` com o conteúdo abaixo. Cada linha introduz um caso inválido independente. A coluna 7 é o indicador de formato fixo; na primeira linha ela contém intencionalmente `X`, enquanto nas demais contém um espaço.

```
000100X   DISPLAY 'invalid indicator'.
000200  DISPLAY @.
000300  DISPLAY #.
000400  MOVE BAD- TO X.
000500  ABCDEFGHIJKLMNOPQRSTUVWXYZ12345.
000600  DISPLAY 'unterminated
000700  PIC .
000800  _BAD.
000900  COPY NO_SUCH_BOOK.
001000  COPY BOOK REPLACING ==A== BY ==B==.
```

Execute:

```sh
cargo run -- lex documents/lexer-errors.cob \
  > /tmp/lexer-errors.tokens 2> /tmp/lexer-errors.diagnostics
```

O comando deve terminar com código diferente de zero, pois os casos contêm erros. O arquivo `/tmp/lexer-errors.diagnostics` contém as mensagens formatadas pelo `miette`; `/tmp/lexer-errors.tokens` contém os tokens que ainda foram reconhecidos durante a recuperação. Registre também o código de saída.

== Casos incluídos

#figure(
  table(
    columns: 3,
    [*Linha*], [*Erro inserido*], [*Diagnóstico esperado*],
    [`000100`], [Indicador inválido na coluna 7], [`cobol::source::invalid_indicator`],
    [`000200`], [Caractere `@` não reconhecido], [`cobol::lexer::illegal_character`],
    [`000300`], [Caractere `#` não reconhecido], [`cobol::lexer::illegal_character`],
    [`000400`], [Identificador terminado em hífen], [`cobol::lexer::trailing_hyphen`],
    [`000500`], [Identificador com 31 caracteres], [`cobol::lexer::identifier_too_long`],
    [`000600`], [Literal string sem aspas de fechamento], [`cobol::lexer::unterminated_string`],
    [`000700`], [Cláusula `PIC` sem picture], [`cobol::lexer::missing_picture`],
    [`000800`], [Caractere `_` no início de uma palavra], [`cobol::lexer::illegal_character`],
    [`000900`], [Copybook inexistente], [`cobol::preprocess::copybook_error`],
    [`001000`], [Cláusula `COPY ... REPLACING` não suportada], [`cobol::preprocess::replacing_unsupported`],
  ),
  caption: [Casos de erro para execução manual; devem ser confirmados pela saída real do programa.],
) <error-cases>

Os dez casos são esperados como diagnósticos de erro. As ocorrências de `@`, `#` e `_` são três instâncias do mesmo código de caractere ilegal. O último caso verifica uma diretiva ainda não suportada e não uma falha de leitura ou de tokenização.

== Saída dos erros

PLACEHOLDER: depois de executar o comando, cole abaixo a saída real de `/tmp/lexer-errors.diagnostics`. Mantenha as mensagens e os locais apresentados pelo programa.

```
PLACEHOLDER: saída dos diagnósticos (dez casos de erro).
```

PLACEHOLDER: informe o código de saída observado e, se desejar, inclua a saída de tokens de `/tmp/lexer-errors.tokens` para mostrar que o lexer continuou após encontrar erros.

= Conclusão

PLACEHOLDER: resuma o que os testes demonstram, as limitações encontradas e quais etapas futuras -- como parser, análise semântica e geração de código -- seriam necessárias para completar o compilador.
