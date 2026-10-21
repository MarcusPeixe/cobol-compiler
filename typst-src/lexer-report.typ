#import "@preview/lambda-notes:0.1.0": *

#show: lambda-notes.with(
  title: [
    Relatório #sym.dot.c Lexer
  ],
  author: [
    #grid(
      align: (left, right),
      columns: 2, gutter: 1em, column-gutter: 30pt,
      "Diego Pedroso de Oliveira", "12684524",
      "Marcus Vinícius Araújo Peixoto", "13729719",
      "Rafael Moura de Almeida", "11225505",
    )
    *Grupo 13*
  ],
  date: [
    *Outubro de 2026*
  ],
  subject: "Implementação e testes de um analisador léxico COBOL",
  keywords: ("COBOL", "lexer", "Rust", "compiladores"),
  color: rgb("#333"),
  show-outline: true,
  outline-title: [Sumário],
  chapter-label: [Seção],
)

#set par(justify: true)

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
  align: (center, left),
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

Os sete programas fornecidos foram adicionados como casos de regressão. O teste de integração executa o leitor de arquivos, o pré-processador, e o lexer para cada exemplo. Ele formata os tokens e compara o resultado com o arquivo de referência correspondente em `tests/lexer-goldens/`. Os _warnings_ são inclusos na saída do programa e nos goldens. Erros de _lexing_ fazem o teste falhar.

Execute a suíte completa na raiz do repositório:

```sh
cargo test
```

Para executar somente o teste dos exemplos:

```sh
cargo test --test lexer_golden_tests
```

A saída completa dos testes é mostrada abaixo:

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
cargo run -- lex cobol-examples/FILE.cob
```

Para guardar separadamente a saída de tokens e os diagnósticos:

```sh
cargo run -- lex cobol-examples/FILE.cob \
  > /tmp/FILE.tokens 2> /tmp/FILE.diagnostics
```

Substitua `FILE` pelo nome do arquivo correspondente. O `stdout` contém os tokens; avisos e erros são escritos no `stderr`. Os fluxos completos podem ser extensos, por isso cada bloco seguinte está reservado para a saída produzida na execução.

=== `prog001_divisoes.cob`

Comando:
`cargo run -- lex cobol-examples/prog001_divisoes.cob`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/cobol-compiler lex .../prog001_divisoes.cob`
.../prog001_divisoes.cob:015:008  KEYWORD IDENTIFICATION
.../prog001_divisoes.cob:015:023  KEYWORD DIVISION
.../prog001_divisoes.cob:015:031  '.'
.../prog001_divisoes.cob:016:008  IDENTIFIER PROGRAM-ID
.../prog001_divisoes.cob:016:018  '.'
.../prog001_divisoes.cob:016:020  IDENTIFIER PROG0001
.../prog001_divisoes.cob:016:028  '.'
.../prog001_divisoes.cob:019:008  IDENTIFIER AUTHOR
.../prog001_divisoes.cob:019:014  '.'
.../prog001_divisoes.cob:019:016  COMMENT-ENTRY "Enio Basso"
.../prog001_divisoes.cob:019:026  '.'
.../prog001_divisoes.cob:020:008  IDENTIFIER INSTALLATION
.../prog001_divisoes.cob:020:020  '.'
.../prog001_divisoes.cob:020:022  COMMENT-ENTRY "ebasso(dot)net"
.../prog001_divisoes.cob:020:036  '.'
.../prog001_divisoes.cob:021:008  IDENTIFIER DATE-WRITTEN
.../prog001_divisoes.cob:021:020  '.'
.../prog001_divisoes.cob:021:022  COMMENT-ENTRY "2018-02-24"
.../prog001_divisoes.cob:021:032  '.'
.../prog001_divisoes.cob:022:008  IDENTIFIER DATE-COMPILED
.../prog001_divisoes.cob:022:021  '.'
.../prog001_divisoes.cob:022:023  COMMENT-ENTRY "2018-02-24"
.../prog001_divisoes.cob:022:033  '.'
.../prog001_divisoes.cob:023:008  IDENTIFIER SECURITY
.../prog001_divisoes.cob:023:016  '.'
.../prog001_divisoes.cob:023:018  COMMENT-ENTRY "O programa usa criptografia XPTO"
.../prog001_divisoes.cob:023:050  '.'
.../prog001_divisoes.cob:024:008  IDENTIFIER REMARKS
.../prog001_divisoes.cob:024:015  '.'
.../prog001_divisoes.cob:024:017  COMMENT-ENTRY "O programa recebe um valor e imprime na tela"
.../prog001_divisoes.cob:024:061  '.'
.../prog001_divisoes.cob:027:008  KEYWORD ENVIRONMENT
.../prog001_divisoes.cob:027:020  KEYWORD DIVISION
.../prog001_divisoes.cob:027:028  '.'
.../prog001_divisoes.cob:029:008  IDENTIFIER CONFIGURATION
.../prog001_divisoes.cob:029:022  KEYWORD SECTION
.../prog001_divisoes.cob:029:029  '.'
.../prog001_divisoes.cob:037:008  IDENTIFIER INPUT-OUTPUT
.../prog001_divisoes.cob:037:021  KEYWORD SECTION
.../prog001_divisoes.cob:037:028  '.'
.../prog001_divisoes.cob:046:008  KEYWORD DATA
.../prog001_divisoes.cob:046:013  KEYWORD DIVISION
.../prog001_divisoes.cob:046:021  '.'
.../prog001_divisoes.cob:053:008  KEYWORD PROCEDURE
.../prog001_divisoes.cob:053:018  KEYWORD DIVISION
.../prog001_divisoes.cob:053:026  '.'
.../prog001_divisoes.cob:054:008  IDENTIFIER MAIN-PROCEDURE
.../prog001_divisoes.cob:054:022  '.'
.../prog001_divisoes.cob:055:013  KEYWORD DISPLAY
.../prog001_divisoes.cob:055:021  STRING "Hello world"
.../prog001_divisoes.cob:056:013  KEYWORD DISPLAY
.../prog001_divisoes.cob:056:021  STRING "Enio Basso - 2"
.../prog001_divisoes.cob:057:013  KEYWORD STOP
.../prog001_divisoes.cob:057:018  KEYWORD RUN
.../prog001_divisoes.cob:057:021  '.'
.../prog001_divisoes.cob:058:008  IDENTIFIER END
.../prog001_divisoes.cob:058:012  IDENTIFIER PROGRAM
.../prog001_divisoes.cob:058:020  IDENTIFIER PROG0001
.../prog001_divisoes.cob:058:028  '.'
```

=== `prog002_operacoes.cob`

Comando:
`cargo run -- lex cobol-examples/prog002_operacoes.cob`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/cobol-compiler lex .../prog002_operacoes.cob`
.../prog002_operacoes.cob:011:008  KEYWORD IDENTIFICATION
.../prog002_operacoes.cob:011:023  KEYWORD DIVISION
.../prog002_operacoes.cob:011:031  '.'
.../prog002_operacoes.cob:012:008  IDENTIFIER PROGRAM-ID
.../prog002_operacoes.cob:012:018  '.'
.../prog002_operacoes.cob:012:020  IDENTIFIER YOUR-PROGRAM-NAME
.../prog002_operacoes.cob:012:037  '.'
.../prog002_operacoes.cob:013:008  KEYWORD DATA
.../prog002_operacoes.cob:013:013  KEYWORD DIVISION
.../prog002_operacoes.cob:013:021  '.'
.../prog002_operacoes.cob:014:008  IDENTIFIER FILE
.../prog002_operacoes.cob:014:013  KEYWORD SECTION
.../prog002_operacoes.cob:014:020  '.'
.../prog002_operacoes.cob:015:008  IDENTIFIER WORKING-STORAGE
.../prog002_operacoes.cob:015:024  KEYWORD SECTION
.../prog002_operacoes.cob:015:031  '.'
.../prog002_operacoes.cob:016:012  NUMBER 77
.../prog002_operacoes.cob:016:015  IDENTIFIER WRK-VALOR1
.../prog002_operacoes.cob:016:032  PICTURE
.../prog002_operacoes.cob:016:036  PICTURE-STRING 9(10)
.../prog002_operacoes.cob:016:042  IDENTIFIER VALUES
.../prog002_operacoes.cob:016:049  NUMBER 10
.../prog002_operacoes.cob:016:051  '.'
.../prog002_operacoes.cob:017:012  NUMBER 77
.../prog002_operacoes.cob:017:015  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:017:032  PICTURE
.../prog002_operacoes.cob:017:036  PICTURE-STRING 9(10)
.../prog002_operacoes.cob:017:042  IDENTIFIER VALUES
.../prog002_operacoes.cob:017:049  IDENTIFIER ZEROS
.../prog002_operacoes.cob:017:054  '.'
.../prog002_operacoes.cob:018:012  NUMBER 77
.../prog002_operacoes.cob:018:015  IDENTIFIER WRK-A
.../prog002_operacoes.cob:018:032  PICTURE
.../prog002_operacoes.cob:018:036  PICTURE-STRING 9(2)
.../prog002_operacoes.cob:018:041  IDENTIFIER VALUES
.../prog002_operacoes.cob:018:048  NUMBER 1
.../prog002_operacoes.cob:018:049  '.'
.../prog002_operacoes.cob:019:012  NUMBER 77
.../prog002_operacoes.cob:019:015  IDENTIFIER WRK-B
.../prog002_operacoes.cob:019:032  PICTURE
.../prog002_operacoes.cob:019:036  PICTURE-STRING 9(2)
.../prog002_operacoes.cob:019:041  IDENTIFIER VALUES
.../prog002_operacoes.cob:019:048  NUMBER 1
.../prog002_operacoes.cob:019:049  '.'
.../prog002_operacoes.cob:020:012  NUMBER 77
.../prog002_operacoes.cob:020:015  IDENTIFIER WRK-C
.../prog002_operacoes.cob:020:032  PICTURE
.../prog002_operacoes.cob:020:036  PICTURE-STRING 9(2)
.../prog002_operacoes.cob:020:041  IDENTIFIER VALUES
.../prog002_operacoes.cob:020:048  IDENTIFIER ZEROS
.../prog002_operacoes.cob:020:053  '.'
.../prog002_operacoes.cob:022:012  NUMBER 01
.../prog002_operacoes.cob:022:015  IDENTIFIER WRK-TOTAIS
.../prog002_operacoes.cob:022:025  '.'
.../prog002_operacoes.cob:023:015  NUMBER 05
.../prog002_operacoes.cob:023:018  IDENTIFIER WRK-TOTAL
.../prog002_operacoes.cob:023:032  PICTURE
.../prog002_operacoes.cob:023:036  PICTURE-STRING 9(10)
.../prog002_operacoes.cob:023:042  IDENTIFIER VALUE
.../prog002_operacoes.cob:023:048  NUMBER 19
.../prog002_operacoes.cob:023:050  '.'
.../prog002_operacoes.cob:025:008  KEYWORD PROCEDURE
.../prog002_operacoes.cob:025:018  KEYWORD DIVISION
.../prog002_operacoes.cob:025:026  '.'
.../prog002_operacoes.cob:026:008  IDENTIFIER MAIN-PROCEDURE
.../prog002_operacoes.cob:026:022  '.'
.../prog002_operacoes.cob:027:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:027:020  STRING "===== start ======="
.../prog002_operacoes.cob:027:041  '.'
.../prog002_operacoes.cob:028:012  KEYWORD MOVE
.../prog002_operacoes.cob:028:017  NUMBER 20
.../prog002_operacoes.cob:028:020  IDENTIFIER TO
.../prog002_operacoes.cob:028:023  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:028:033  '.'
.../prog002_operacoes.cob:029:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:029:020  STRING "MOVE 20 TO WRK-VALOR2 -->"
.../prog002_operacoes.cob:029:048  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:029:058  '.'
.../prog002_operacoes.cob:031:012  KEYWORD MOVE
.../prog002_operacoes.cob:031:017  IDENTIFIER WRK-VALOR1
.../prog002_operacoes.cob:031:028  IDENTIFIER TO
.../prog002_operacoes.cob:031:031  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:031:041  '.'
.../prog002_operacoes.cob:032:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:032:020  STRING "MOVE WRK-VALOR1 TO WRK-VALOR2 -->"
.../prog002_operacoes.cob:032:056  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:032:066  '.'
.../prog002_operacoes.cob:034:012  KEYWORD MOVE
.../prog002_operacoes.cob:034:017  NUMBER 70
.../prog002_operacoes.cob:034:020  IDENTIFIER TO
.../prog002_operacoes.cob:034:023  IDENTIFIER WRK-VALOR1
.../prog002_operacoes.cob:034:034  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:034:044  '.'
.../prog002_operacoes.cob:035:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:035:020  STRING "MOVE 70 TO WRK-VALOR1 WRK-VALOR2 -->"
.../prog002_operacoes.cob:035:059  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:035:069  '.'
.../prog002_operacoes.cob:037:012  KEYWORD MOVE
.../prog002_operacoes.cob:037:017  IDENTIFIER WRK-TOTAL
.../prog002_operacoes.cob:037:027  IDENTIFIER OF
.../prog002_operacoes.cob:037:030  IDENTIFIER WRK-TOTAIS
.../prog002_operacoes.cob:037:041  IDENTIFIER TO
.../prog002_operacoes.cob:037:044  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:037:054  '.'
.../prog002_operacoes.cob:038:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:038:020  STRING "MOVE WRK-TOTAL OF WRK-TOTAIS TO WRK-VALOR2. -->"
.../prog002_operacoes.cob:039:022  IDENTIFIER WRK-VALOR2
.../prog002_operacoes.cob:039:032  '.'
.../prog002_operacoes.cob:041:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:041:020  STRING "--------"
.../prog002_operacoes.cob:041:030  '.'
.../prog002_operacoes.cob:042:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:042:020  STRING "A = 1, B = 1, C = 1"
.../prog002_operacoes.cob:042:041  '.'
.../prog002_operacoes.cob:044:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:044:020  STRING " "
.../prog002_operacoes.cob:044:023  '.'
.../prog002_operacoes.cob:045:012  IDENTIFIER ADD
.../prog002_operacoes.cob:045:016  NUMBER 1
.../prog002_operacoes.cob:045:018  NUMBER 1
.../prog002_operacoes.cob:045:020  IDENTIFIER TO
.../prog002_operacoes.cob:045:023  IDENTIFIER WRK-A
.../prog002_operacoes.cob:045:028  '.'
.../prog002_operacoes.cob:046:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:046:020  STRING "ADD B C TO A (A = B + C + A) -->"
.../prog002_operacoes.cob:046:055  IDENTIFIER WRK-A
.../prog002_operacoes.cob:046:060  '.'
.../prog002_operacoes.cob:048:012  KEYWORD MOVE
.../prog002_operacoes.cob:048:017  NUMBER 1
.../prog002_operacoes.cob:048:019  IDENTIFIER TO
.../prog002_operacoes.cob:048:022  IDENTIFIER WRK-A
.../prog002_operacoes.cob:048:027  '.'
.../prog002_operacoes.cob:049:012  IDENTIFIER ADD
.../prog002_operacoes.cob:049:016  NUMBER 1
.../prog002_operacoes.cob:049:018  IDENTIFIER TO
.../prog002_operacoes.cob:049:021  IDENTIFIER WRK-A
.../prog002_operacoes.cob:049:027  IDENTIFIER WRK-B
.../prog002_operacoes.cob:049:032  '.'
.../prog002_operacoes.cob:050:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:050:020  STRING " "
.../prog002_operacoes.cob:050:023  '.'
.../prog002_operacoes.cob:051:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:051:020  STRING "ADD 1 TO A B (A = A + 1; B = B + 1) -->"
.../prog002_operacoes.cob:052:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:052:020  STRING "A= "
.../prog002_operacoes.cob:052:026  IDENTIFIER WRK-A
.../prog002_operacoes.cob:052:032  STRING " , B= "
.../prog002_operacoes.cob:052:041  IDENTIFIER WRK-B
.../prog002_operacoes.cob:052:046  '.'
.../prog002_operacoes.cob:054:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:054:020  STRING " "
.../prog002_operacoes.cob:054:023  '.'
.../prog002_operacoes.cob:055:012  IDENTIFIER ADD
.../prog002_operacoes.cob:055:016  NUMBER 1
.../prog002_operacoes.cob:055:018  NUMBER 1
.../prog002_operacoes.cob:055:020  IDENTIFIER GIVING
.../prog002_operacoes.cob:055:027  IDENTIFIER WRK-A
.../prog002_operacoes.cob:055:032  '.'
.../prog002_operacoes.cob:056:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:056:020  STRING "ADD B C GIVING A (A = B + C) -->"
.../prog002_operacoes.cob:056:055  IDENTIFIER WRK-A
.../prog002_operacoes.cob:056:060  '.'
.../prog002_operacoes.cob:058:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:058:020  STRING "============ SUBTRACT ============= "
.../prog002_operacoes.cob:058:058  '.'
.../prog002_operacoes.cob:059:012  KEYWORD MOVE
.../prog002_operacoes.cob:059:017  NUMBER 99
.../prog002_operacoes.cob:059:020  IDENTIFIER TO
.../prog002_operacoes.cob:059:023  IDENTIFIER WRK-A
.../prog002_operacoes.cob:059:028  '.'
.../prog002_operacoes.cob:060:012  KEYWORD MOVE
.../prog002_operacoes.cob:060:017  NUMBER 0
.../prog002_operacoes.cob:060:019  IDENTIFIER TO
.../prog002_operacoes.cob:060:022  IDENTIFIER WRK-B
.../prog002_operacoes.cob:060:027  '.'
.../prog002_operacoes.cob:061:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:061:020  STRING "A= 99, B= 0"
.../prog002_operacoes.cob:061:033  '.'
.../prog002_operacoes.cob:062:012  IDENTIFIER SUBTRACT
.../prog002_operacoes.cob:062:021  NUMBER 30
.../prog002_operacoes.cob:062:024  NUMBER 23
.../prog002_operacoes.cob:062:027  IDENTIFIER FROM
.../prog002_operacoes.cob:062:032  IDENTIFIER WRK-A
.../prog002_operacoes.cob:063:027  IDENTIFIER GIVING
.../prog002_operacoes.cob:063:034  IDENTIFIER WRK-B
.../prog002_operacoes.cob:063:039  '.'
.../prog002_operacoes.cob:064:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:064:020  STRING "SUBTRACT 30 23 FROM A GIVING B.-->"
.../prog002_operacoes.cob:065:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:065:020  STRING "A= "
.../prog002_operacoes.cob:065:026  IDENTIFIER WRK-A
.../prog002_operacoes.cob:065:032  STRING " , B= "
.../prog002_operacoes.cob:065:041  IDENTIFIER WRK-B
.../prog002_operacoes.cob:065:046  '.'
.../prog002_operacoes.cob:067:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:067:020  STRING "============ MULTIPLY ============= "
.../prog002_operacoes.cob:067:058  '.'
.../prog002_operacoes.cob:068:012  KEYWORD MOVE
.../prog002_operacoes.cob:068:017  NUMBER 40
.../prog002_operacoes.cob:068:020  IDENTIFIER TO
.../prog002_operacoes.cob:068:023  IDENTIFIER WRK-A
.../prog002_operacoes.cob:068:028  '.'
.../prog002_operacoes.cob:069:012  KEYWORD MOVE
.../prog002_operacoes.cob:069:017  NUMBER 0
.../prog002_operacoes.cob:069:019  IDENTIFIER TO
.../prog002_operacoes.cob:069:022  IDENTIFIER WRK-B
.../prog002_operacoes.cob:069:027  '.'
.../prog002_operacoes.cob:070:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:070:020  STRING "A= 40, B= 0"
.../prog002_operacoes.cob:070:033  '.'
.../prog002_operacoes.cob:071:012  IDENTIFIER MULTIPLY
.../prog002_operacoes.cob:071:021  NUMBER 2
.../prog002_operacoes.cob:071:023  IDENTIFIER BY
.../prog002_operacoes.cob:071:026  IDENTIFIER WRK-A
.../prog002_operacoes.cob:071:031  '.'
.../prog002_operacoes.cob:072:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:072:020  STRING "MULTIPLY A BY 2 -->"
.../prog002_operacoes.cob:073:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:073:020  STRING "A= "
.../prog002_operacoes.cob:073:026  IDENTIFIER WRK-A
.../prog002_operacoes.cob:073:032  STRING " , B= "
.../prog002_operacoes.cob:073:041  IDENTIFIER WRK-B
.../prog002_operacoes.cob:073:046  '.'
.../prog002_operacoes.cob:075:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:075:020  STRING "============ MULTIPLY ============= "
.../prog002_operacoes.cob:075:058  '.'
.../prog002_operacoes.cob:076:012  KEYWORD MOVE
.../prog002_operacoes.cob:076:017  NUMBER 40
.../prog002_operacoes.cob:076:020  IDENTIFIER TO
.../prog002_operacoes.cob:076:023  IDENTIFIER WRK-A
.../prog002_operacoes.cob:076:028  '.'
.../prog002_operacoes.cob:077:012  KEYWORD MOVE
.../prog002_operacoes.cob:077:017  NUMBER 0
.../prog002_operacoes.cob:077:019  IDENTIFIER TO
.../prog002_operacoes.cob:077:022  IDENTIFIER WRK-B
.../prog002_operacoes.cob:077:027  '.'
.../prog002_operacoes.cob:078:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:078:020  STRING "A= 40, B= 0"
.../prog002_operacoes.cob:078:033  '.'
.../prog002_operacoes.cob:079:012  IDENTIFIER MULTIPLY
.../prog002_operacoes.cob:079:021  NUMBER 2
.../prog002_operacoes.cob:079:023  IDENTIFIER BY
.../prog002_operacoes.cob:079:026  IDENTIFIER WRK-A
.../prog002_operacoes.cob:079:032  IDENTIFIER GIVING
.../prog002_operacoes.cob:079:039  IDENTIFIER WRK-B
.../prog002_operacoes.cob:079:044  '.'
.../prog002_operacoes.cob:080:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:080:020  STRING "MULTIPLY A BY 2 GIVING B.-->"
.../prog002_operacoes.cob:081:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:081:020  STRING "A= "
.../prog002_operacoes.cob:081:026  IDENTIFIER WRK-A
.../prog002_operacoes.cob:081:032  STRING " , B= "
.../prog002_operacoes.cob:081:041  IDENTIFIER WRK-B
.../prog002_operacoes.cob:081:046  '.'
.../prog002_operacoes.cob:083:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:083:020  STRING "============ DIVIDE ============="
.../prog002_operacoes.cob:083:055  '.'
.../prog002_operacoes.cob:084:012  KEYWORD MOVE
.../prog002_operacoes.cob:084:017  NUMBER 81
.../prog002_operacoes.cob:084:020  IDENTIFIER TO
.../prog002_operacoes.cob:084:023  IDENTIFIER WRK-A
.../prog002_operacoes.cob:084:028  '.'
.../prog002_operacoes.cob:085:012  KEYWORD MOVE
.../prog002_operacoes.cob:085:017  NUMBER 0
.../prog002_operacoes.cob:085:019  IDENTIFIER TO
.../prog002_operacoes.cob:085:022  IDENTIFIER WRK-B
.../prog002_operacoes.cob:085:027  '.'
.../prog002_operacoes.cob:086:012  KEYWORD MOVE
.../prog002_operacoes.cob:086:017  NUMBER 0
.../prog002_operacoes.cob:086:019  IDENTIFIER TO
.../prog002_operacoes.cob:086:022  IDENTIFIER WRK-C
.../prog002_operacoes.cob:086:027  '.'
.../prog002_operacoes.cob:087:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:087:020  STRING "A= 80, B= 0"
.../prog002_operacoes.cob:087:033  '.'
.../prog002_operacoes.cob:088:012  IDENTIFIER DIVIDE
.../prog002_operacoes.cob:088:019  IDENTIFIER WRK-A
.../prog002_operacoes.cob:088:025  IDENTIFIER BY
.../prog002_operacoes.cob:088:028  NUMBER 2
.../prog002_operacoes.cob:088:030  IDENTIFIER GIVING
.../prog002_operacoes.cob:088:037  IDENTIFIER WRK-B
.../prog002_operacoes.cob:088:043  IDENTIFIER REMAINDER
.../prog002_operacoes.cob:088:053  IDENTIFIER WRK-C
.../prog002_operacoes.cob:088:058  '.'
.../prog002_operacoes.cob:089:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:089:020  STRING "DIVIDE A BY 2 GIVING B REMAINDER C .-->"
.../prog002_operacoes.cob:090:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:090:020  STRING "A= "
.../prog002_operacoes.cob:090:026  IDENTIFIER WRK-A
.../prog002_operacoes.cob:090:032  STRING " , B= "
.../prog002_operacoes.cob:090:041  IDENTIFIER WRK-B
.../prog002_operacoes.cob:090:047  STRING " , C= "
.../prog002_operacoes.cob:090:056  IDENTIFIER WRK-C
.../prog002_operacoes.cob:090:061  '.'
.../prog002_operacoes.cob:093:012  KEYWORD DISPLAY
.../prog002_operacoes.cob:093:020  STRING " "
.../prog002_operacoes.cob:093:023  '.'
.../prog002_operacoes.cob:094:012  KEYWORD MOVE
.../prog002_operacoes.cob:094:017  NUMBER 99
.../prog002_operacoes.cob:094:020  IDENTIFIER TO
.../prog002_operacoes.cob:094:023  IDENTIFIER WRK-A
.../prog002_operacoes.cob:094:028  '.'
.../prog002_operacoes.cob:095:012  IDENTIFIER ADD
.../prog002_operacoes.cob:095:016  NUMBER 1
.../prog002_operacoes.cob:095:018  IDENTIFIER TO
.../prog002_operacoes.cob:095:021  IDENTIFIER WRK-A
.../prog002_operacoes.cob:096:012  IDENTIFIER ON
.../prog002_operacoes.cob:096:015  IDENTIFIER SIZE
.../prog002_operacoes.cob:096:020  IDENTIFIER ERROR
.../prog002_operacoes.cob:097:015  KEYWORD DISPLAY
.../prog002_operacoes.cob:097:023  STRING "ON SIZE ERROR -->"
.../prog002_operacoes.cob:097:043  IDENTIFIER WRK-A
.../prog002_operacoes.cob:097:048  '.'
.../prog002_operacoes.cob:099:013  KEYWORD STOP
.../prog002_operacoes.cob:099:018  KEYWORD RUN
.../prog002_operacoes.cob:099:021  '.'
.../prog002_operacoes.cob:100:008  IDENTIFIER END
.../prog002_operacoes.cob:100:012  IDENTIFIER PROGRAM
.../prog002_operacoes.cob:100:020  IDENTIFIER YOUR-PROGRAM-NAME
.../prog002_operacoes.cob:100:037  '.'
```

=== `prog003_calculadora.cob`

Comando:
`cargo run -- lex cobol-examples/prog003_calculadora.cob`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/cobol-compiler lex .../prog003_calculadora.cob`
.../prog003_calculadora.cob:011:008  KEYWORD IDENTIFICATION
.../prog003_calculadora.cob:011:023  KEYWORD DIVISION
.../prog003_calculadora.cob:011:031  '.'
.../prog003_calculadora.cob:012:008  IDENTIFIER PROGRAM-ID
.../prog003_calculadora.cob:012:018  '.'
.../prog003_calculadora.cob:012:020  IDENTIFIER CALCULADORA
.../prog003_calculadora.cob:012:031  '.'
.../prog003_calculadora.cob:013:008  KEYWORD DATA
.../prog003_calculadora.cob:013:013  KEYWORD DIVISION
.../prog003_calculadora.cob:013:021  '.'
.../prog003_calculadora.cob:014:008  IDENTIFIER FILE
.../prog003_calculadora.cob:014:013  KEYWORD SECTION
.../prog003_calculadora.cob:014:020  '.'
.../prog003_calculadora.cob:015:008  IDENTIFIER WORKING-STORAGE
.../prog003_calculadora.cob:015:024  KEYWORD SECTION
.../prog003_calculadora.cob:015:031  '.'
.../prog003_calculadora.cob:016:012  NUMBER 77
.../prog003_calculadora.cob:016:015  IDENTIFIER WKS-NUM-1
.../prog003_calculadora.cob:016:035  PICTURE
.../prog003_calculadora.cob:016:039  PICTURE-STRING S9(05)V99
.../prog003_calculadora.cob:016:052  IDENTIFIER VALUES
.../prog003_calculadora.cob:016:059  NUMBER 0
.../prog003_calculadora.cob:016:060  '.'
.../prog003_calculadora.cob:017:012  NUMBER 77
.../prog003_calculadora.cob:017:015  IDENTIFIER WKS-NUM-2
.../prog003_calculadora.cob:017:035  PICTURE
.../prog003_calculadora.cob:017:039  PICTURE-STRING S9(05)V9(02)
.../prog003_calculadora.cob:017:052  IDENTIFIER VALUES
.../prog003_calculadora.cob:017:059  NUMBER 0
.../prog003_calculadora.cob:017:060  '.'
.../prog003_calculadora.cob:018:012  NUMBER 77
.../prog003_calculadora.cob:018:015  IDENTIFIER WKS-RESULTADO
.../prog003_calculadora.cob:018:035  PICTURE
.../prog003_calculadora.cob:018:039  PICTURE-STRING S9(10)V99
.../prog003_calculadora.cob:018:052  IDENTIFIER VALUES
.../prog003_calculadora.cob:018:059  IDENTIFIER ZEROS
.../prog003_calculadora.cob:018:064  '.'
.../prog003_calculadora.cob:019:012  NUMBER 77
.../prog003_calculadora.cob:019:015  IDENTIFIER WKS-RESTO
.../prog003_calculadora.cob:019:035  PICTURE
.../prog003_calculadora.cob:019:039  PICTURE-STRING 9(2)
.../prog003_calculadora.cob:019:052  IDENTIFIER VALUES
.../prog003_calculadora.cob:019:059  IDENTIFIER ZEROS
.../prog003_calculadora.cob:019:064  '.'
.../prog003_calculadora.cob:020:012  NUMBER 77
.../prog003_calculadora.cob:020:015  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:020:035  PICTURE
.../prog003_calculadora.cob:020:039  PICTURE-STRING A(01)
.../prog003_calculadora.cob:020:044  '.'
.../prog003_calculadora.cob:021:012  NUMBER 77
.../prog003_calculadora.cob:021:015  IDENTIFIER WKS-SAIR
.../prog003_calculadora.cob:021:035  PICTURE
.../prog003_calculadora.cob:021:039  PICTURE-STRING 9(1)
.../prog003_calculadora.cob:021:052  IDENTIFIER VALUES
.../prog003_calculadora.cob:021:059  IDENTIFIER ZEROS
.../prog003_calculadora.cob:021:064  '.'
.../prog003_calculadora.cob:022:012  NUMBER 77
.../prog003_calculadora.cob:022:015  IDENTIFIER WKS-RESULT-FORMAT
.../prog003_calculadora.cob:022:035  PICTURE
.../prog003_calculadora.cob:022:039  PICTURE-STRING -ZZZZZZZZZ9.99
.../prog003_calculadora.cob:022:053  '.'
.../prog003_calculadora.cob:024:008  KEYWORD PROCEDURE
.../prog003_calculadora.cob:024:018  KEYWORD DIVISION
.../prog003_calculadora.cob:024:026  '.'
.../prog003_calculadora.cob:025:008  IDENTIFIER MAIN-PROCEDURE
.../prog003_calculadora.cob:025:022  '.'
.../prog003_calculadora.cob:027:012  KEYWORD PERFORM
.../prog003_calculadora.cob:027:020  IDENTIFIER 00000-CALCULADORA
.../prog003_calculadora.cob:027:038  IDENTIFIER UNTIL
.../prog003_calculadora.cob:027:044  IDENTIFIER WKS-SAIR
.../prog003_calculadora.cob:027:053  '='
.../prog003_calculadora.cob:027:055  NUMBER 2
.../prog003_calculadora.cob:027:056  '.'
.../prog003_calculadora.cob:029:012  KEYWORD STOP
.../prog003_calculadora.cob:029:017  KEYWORD RUN
.../prog003_calculadora.cob:029:020  '.'
.../prog003_calculadora.cob:031:008  IDENTIFIER 00000-CALCULADORA
.../prog003_calculadora.cob:031:025  '.'
.../prog003_calculadora.cob:032:012  KEYWORD DISPLAY
.../prog003_calculadora.cob:032:020  STRING "============ CALCULADORA ============= "
.../prog003_calculadora.cob:032:061  '.'
.../prog003_calculadora.cob:033:012  KEYWORD DISPLAY
.../prog003_calculadora.cob:033:020  STRING "1-. USAR CALCULADORA, 2-. SAIR"
.../prog003_calculadora.cob:033:052  '.'
.../prog003_calculadora.cob:034:012  KEYWORD ACCEPT
.../prog003_calculadora.cob:034:019  IDENTIFIER WKS-SAIR
.../prog003_calculadora.cob:034:027  '.'
.../prog003_calculadora.cob:036:012  KEYWORD IF
.../prog003_calculadora.cob:036:015  IDENTIFIER WKS-SAIR
.../prog003_calculadora.cob:036:024  '='
.../prog003_calculadora.cob:036:026  NUMBER 1
.../prog003_calculadora.cob:037:015  KEYWORD PERFORM
.../prog003_calculadora.cob:037:023  IDENTIFIER 00001-PEDIR-DADOS
.../prog003_calculadora.cob:038:015  KEYWORD PERFORM
.../prog003_calculadora.cob:038:023  IDENTIFIER 00002-EXECUTAR-OPERACOES
.../prog003_calculadora.cob:039:012  IDENTIFIER END-IF
.../prog003_calculadora.cob:039:018  '.'
.../prog003_calculadora.cob:041:008  IDENTIFIER 00001-PEDIR-DADOS
.../prog003_calculadora.cob:041:025  '.'
.../prog003_calculadora.cob:042:012  KEYWORD DISPLAY
.../prog003_calculadora.cob:042:020  STRING "OPERACAO: "
.../prog003_calculadora.cob:042:032  '.'
.../prog003_calculadora.cob:043:012  KEYWORD DISPLAY
.../prog003_calculadora.cob:043:020  STRING "(S)oma, s(U)btracao, (M)ultiplicacao e (D)ivisao"
.../prog003_calculadora.cob:043:070  '.'
.../prog003_calculadora.cob:044:012  KEYWORD DISPLAY
.../prog003_calculadora.cob:044:020  STRING "(P)otencia"
.../prog003_calculadora.cob:044:032  '.'
.../prog003_calculadora.cob:045:012  KEYWORD DISPLAY
.../prog003_calculadora.cob:045:020  STRING "==>"
.../prog003_calculadora.cob:045:025  '.'
.../prog003_calculadora.cob:046:012  KEYWORD ACCEPT
.../prog003_calculadora.cob:046:019  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:046:028  '.'
.../prog003_calculadora.cob:048:012  KEYWORD DISPLAY
.../prog003_calculadora.cob:048:020  STRING "VALOR 1:"
.../prog003_calculadora.cob:048:030  '.'
.../prog003_calculadora.cob:049:012  KEYWORD ACCEPT
.../prog003_calculadora.cob:049:019  IDENTIFIER WKS-NUM-1
.../prog003_calculadora.cob:049:028  '.'
.../prog003_calculadora.cob:051:012  KEYWORD DISPLAY
.../prog003_calculadora.cob:051:020  STRING "VALOR 2:"
.../prog003_calculadora.cob:051:030  '.'
.../prog003_calculadora.cob:052:012  KEYWORD ACCEPT
.../prog003_calculadora.cob:052:019  IDENTIFIER WKS-NUM-2
.../prog003_calculadora.cob:052:028  '.'
.../prog003_calculadora.cob:054:008  IDENTIFIER 00002-EXECUTAR-OPERACOES
.../prog003_calculadora.cob:054:032  '.'
.../prog003_calculadora.cob:055:012  KEYWORD MOVE
.../prog003_calculadora.cob:055:017  IDENTIFIER FUNCTION
.../prog003_calculadora.cob:055:026  IDENTIFIER UPPER-CASE
.../prog003_calculadora.cob:055:036  '('
.../prog003_calculadora.cob:055:037  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:055:046  ')'
.../prog003_calculadora.cob:055:048  IDENTIFIER TO
.../prog003_calculadora.cob:055:051  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:055:060  '.'
.../prog003_calculadora.cob:056:012  KEYWORD IF
.../prog003_calculadora.cob:056:015  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:056:025  '='
.../prog003_calculadora.cob:056:027  STRING "S"
.../prog003_calculadora.cob:056:031  IDENTIFIER OR
.../prog003_calculadora.cob:056:034  STRING "s"
.../prog003_calculadora.cob:057:015  IDENTIFIER ADD
.../prog003_calculadora.cob:057:019  IDENTIFIER WKS-NUM-1
.../prog003_calculadora.cob:057:029  IDENTIFIER TO
.../prog003_calculadora.cob:057:032  IDENTIFIER WKS-NUM-2
.../prog003_calculadora.cob:057:042  IDENTIFIER GIVING
.../prog003_calculadora.cob:057:049  IDENTIFIER WKS-RESULTADO
.../prog003_calculadora.cob:058:012  KEYWORD ELSE
.../prog003_calculadora.cob:059:012  KEYWORD IF
.../prog003_calculadora.cob:059:015  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:059:025  '='
.../prog003_calculadora.cob:059:027  STRING "U"
.../prog003_calculadora.cob:060:015  IDENTIFIER SUBTRACT
.../prog003_calculadora.cob:060:024  IDENTIFIER WKS-NUM-1
.../prog003_calculadora.cob:060:034  IDENTIFIER FROM
.../prog003_calculadora.cob:060:039  IDENTIFIER WKS-NUM-2
.../prog003_calculadora.cob:060:049  IDENTIFIER GIVING
.../prog003_calculadora.cob:060:056  IDENTIFIER WKS-RESULTADO
.../prog003_calculadora.cob:061:012  KEYWORD ELSE
.../prog003_calculadora.cob:062:012  KEYWORD IF
.../prog003_calculadora.cob:062:015  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:062:025  '='
.../prog003_calculadora.cob:062:027  STRING "M"
.../prog003_calculadora.cob:063:015  IDENTIFIER MULTIPLY
.../prog003_calculadora.cob:063:024  IDENTIFIER WKS-NUM-1
.../prog003_calculadora.cob:063:034  IDENTIFIER BY
.../prog003_calculadora.cob:063:037  IDENTIFIER WKS-NUM-2
.../prog003_calculadora.cob:063:047  IDENTIFIER GIVING
.../prog003_calculadora.cob:063:054  IDENTIFIER WKS-RESULTADO
.../prog003_calculadora.cob:064:012  KEYWORD ELSE
.../prog003_calculadora.cob:065:012  KEYWORD IF
.../prog003_calculadora.cob:065:015  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:065:025  '='
.../prog003_calculadora.cob:065:027  STRING "D"
.../prog003_calculadora.cob:066:015  IDENTIFIER DIVIDE
.../prog003_calculadora.cob:066:022  IDENTIFIER WKS-NUM-1
.../prog003_calculadora.cob:066:032  IDENTIFIER BY
.../prog003_calculadora.cob:066:035  IDENTIFIER WKS-NUM-2
.../prog003_calculadora.cob:066:045  IDENTIFIER GIVING
.../prog003_calculadora.cob:066:052  IDENTIFIER WKS-RESULTADO
.../prog003_calculadora.cob:067:042  IDENTIFIER REMAINDER
.../prog003_calculadora.cob:067:052  IDENTIFIER WKS-RESTO
.../prog003_calculadora.cob:068:012  KEYWORD ELSE
.../prog003_calculadora.cob:069:012  KEYWORD IF
.../prog003_calculadora.cob:069:015  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:069:025  '='
.../prog003_calculadora.cob:069:027  STRING "P"
.../prog003_calculadora.cob:070:015  IDENTIFIER COMPUTE
.../prog003_calculadora.cob:070:023  IDENTIFIER WKS-RESULTADO
.../prog003_calculadora.cob:070:037  '='
.../prog003_calculadora.cob:070:039  IDENTIFIER WKS-NUM-1
.../prog003_calculadora.cob:070:049  '**'
.../prog003_calculadora.cob:070:052  IDENTIFIER WKS-NUM-2
.../prog003_calculadora.cob:071:012  KEYWORD ELSE
.../prog003_calculadora.cob:072:015  KEYWORD DISPLAY
.../prog003_calculadora.cob:072:023  STRING "OPCAO INVALIDA"
.../prog003_calculadora.cob:073:012  IDENTIFIER END-IF
.../prog003_calculadora.cob:074:012  IDENTIFIER END-IF
.../prog003_calculadora.cob:075:012  IDENTIFIER END-IF
.../prog003_calculadora.cob:076:012  IDENTIFIER END-IF
.../prog003_calculadora.cob:077:012  IDENTIFIER END-IF
.../prog003_calculadora.cob:077:018  '.'
.../prog003_calculadora.cob:078:012  KEYWORD IF
.../prog003_calculadora.cob:078:015  IDENTIFIER WKS-OPCAO
.../prog003_calculadora.cob:078:025  '='
.../prog003_calculadora.cob:078:027  STRING "S"
.../prog003_calculadora.cob:078:031  IDENTIFIER OR
.../prog003_calculadora.cob:078:034  STRING "U"
.../prog003_calculadora.cob:078:038  IDENTIFIER OR
.../prog003_calculadora.cob:078:041  STRING "M"
.../prog003_calculadora.cob:078:045  IDENTIFIER OR
.../prog003_calculadora.cob:078:048  STRING "D"
.../prog003_calculadora.cob:078:052  IDENTIFIER OR
.../prog003_calculadora.cob:078:055  STRING "P"
.../prog003_calculadora.cob:079:016  KEYWORD MOVE
.../prog003_calculadora.cob:079:021  IDENTIFIER WKS-RESULTADO
.../prog003_calculadora.cob:079:035  IDENTIFIER TO
.../prog003_calculadora.cob:079:038  IDENTIFIER WKS-RESULT-FORMAT
.../prog003_calculadora.cob:080:015  KEYWORD DISPLAY
.../prog003_calculadora.cob:080:023  STRING "Resultado: "
.../prog003_calculadora.cob:080:037  IDENTIFIER WKS-RESULT-FORMAT
.../prog003_calculadora.cob:081:012  IDENTIFIER END-IF
.../prog003_calculadora.cob:081:018  '.'
.../prog003_calculadora.cob:083:008  IDENTIFIER END
.../prog003_calculadora.cob:083:012  IDENTIFIER PROGRAM
.../prog003_calculadora.cob:083:020  IDENTIFIER CALCULADORA
.../prog003_calculadora.cob:083:031  '.'
```

=== `prog004_folha_pgto.cob`

Comando:
`cargo run -- lex cobol-examples/prog004_folha_pgto.cob`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/cobol-compiler lex .../prog004_folha_pgto.cob`
.../prog004_folha_pgto.cob:011:008  KEYWORD IDENTIFICATION
.../prog004_folha_pgto.cob:011:023  KEYWORD DIVISION
.../prog004_folha_pgto.cob:011:031  '.'
.../prog004_folha_pgto.cob:012:008  IDENTIFIER PROGRAM-ID
.../prog004_folha_pgto.cob:012:018  '.'
.../prog004_folha_pgto.cob:012:020  IDENTIFIER FOLHA-PGTO
.../prog004_folha_pgto.cob:012:030  '.'
.../prog004_folha_pgto.cob:013:008  KEYWORD DATA
.../prog004_folha_pgto.cob:013:013  KEYWORD DIVISION
.../prog004_folha_pgto.cob:013:021  '.'
.../prog004_folha_pgto.cob:014:008  IDENTIFIER FILE
.../prog004_folha_pgto.cob:014:013  KEYWORD SECTION
.../prog004_folha_pgto.cob:014:020  '.'
.../prog004_folha_pgto.cob:015:008  IDENTIFIER WORKING-STORAGE
.../prog004_folha_pgto.cob:015:024  KEYWORD SECTION
.../prog004_folha_pgto.cob:015:031  '.'
.../prog004_folha_pgto.cob:017:008  NUMBER 01
.../prog004_folha_pgto.cob:017:011  IDENTIFIER TAB-TRABALHADOR
.../prog004_folha_pgto.cob:017:026  '.'
.../prog004_folha_pgto.cob:018:011  NUMBER 02
.../prog004_folha_pgto.cob:018:014  IDENTIFIER TRABALHADOR
.../prog004_folha_pgto.cob:018:026  IDENTIFIER OCCURS
.../prog004_folha_pgto.cob:018:033  NUMBER 3
.../prog004_folha_pgto.cob:018:035  IDENTIFIER TIMES
.../prog004_folha_pgto.cob:018:041  IDENTIFIER INDEXED
.../prog004_folha_pgto.cob:018:049  IDENTIFIER BY
.../prog004_folha_pgto.cob:018:052  IDENTIFIER TID
.../prog004_folha_pgto.cob:018:055  '.'
.../prog004_folha_pgto.cob:019:014  NUMBER 03
.../prog004_folha_pgto.cob:019:017  IDENTIFIER NOME
.../prog004_folha_pgto.cob:019:033  PICTURE
.../prog004_folha_pgto.cob:019:037  PICTURE-STRING X(5)
.../prog004_folha_pgto.cob:019:041  '.'
.../prog004_folha_pgto.cob:020:014  NUMBER 03
.../prog004_folha_pgto.cob:020:017  IDENTIFIER SALARIO
.../prog004_folha_pgto.cob:020:033  PICTURE
.../prog004_folha_pgto.cob:020:037  PICTURE-STRING 9(5)V99
.../prog004_folha_pgto.cob:020:044  '.'
.../prog004_folha_pgto.cob:021:014  NUMBER 03
.../prog004_folha_pgto.cob:021:017  IDENTIFIER DPTO
.../prog004_folha_pgto.cob:021:033  PICTURE
.../prog004_folha_pgto.cob:021:037  PICTURE-STRING X(2)
.../prog004_folha_pgto.cob:021:042  IDENTIFIER VALUES
.../prog004_folha_pgto.cob:021:049  IDENTIFIER SPACES
.../prog004_folha_pgto.cob:021:056  IDENTIFIER OCCURS
.../prog004_folha_pgto.cob:021:063  NUMBER 2
.../prog004_folha_pgto.cob:021:065  IDENTIFIER TIMES
.../prog004_folha_pgto.cob:021:070  '.'
.../prog004_folha_pgto.cob:023:008  NUMBER 77
.../prog004_folha_pgto.cob:023:011  IDENTIFIER IDX
.../prog004_folha_pgto.cob:023:015  PICTURE
.../prog004_folha_pgto.cob:023:019  PICTURE-STRING 9(1)
.../prog004_folha_pgto.cob:023:023  '.'
.../prog004_folha_pgto.cob:025:008  KEYWORD PROCEDURE
.../prog004_folha_pgto.cob:025:018  KEYWORD DIVISION
.../prog004_folha_pgto.cob:025:026  '.'
.../prog004_folha_pgto.cob:026:008  IDENTIFIER MAIN-PROCEDURE
.../prog004_folha_pgto.cob:026:022  '.'
.../prog004_folha_pgto.cob:027:013  KEYWORD MOVE
.../prog004_folha_pgto.cob:027:018  STRING "JOSEA1111111A1A4MARIA2222222A2  FABIO3333333A2A3"
.../prog004_folha_pgto.cob:027:069  IDENTIFIER TO
.../prog004_folha_pgto.cob:027:072  IDENTIFIER T
.../prog004_folha_pgto.cob:028:008  IDENTIFIER AB-TRABALHADOR
.../prog004_folha_pgto.cob:028:022  '.'
.../prog004_folha_pgto.cob:029:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:029:021  STRING " "
.../prog004_folha_pgto.cob:029:024  '.'
.../prog004_folha_pgto.cob:030:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:030:021  STRING "====TAB-TRABALHADOR===="
.../prog004_folha_pgto.cob:030:046  '.'
.../prog004_folha_pgto.cob:031:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:031:021  STRING "INITIAL VALUE"
.../prog004_folha_pgto.cob:031:036  '.'
.../prog004_folha_pgto.cob:032:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:032:021  IDENTIFIER TAB-TRABALHADOR
.../prog004_folha_pgto.cob:032:036  '.'
.../prog004_folha_pgto.cob:034:013  KEYWORD MOVE
.../prog004_folha_pgto.cob:034:018  NUMBER 1
.../prog004_folha_pgto.cob:034:020  IDENTIFIER TO
.../prog004_folha_pgto.cob:034:023  IDENTIFIER IDX
.../prog004_folha_pgto.cob:034:026  '.'
.../prog004_folha_pgto.cob:035:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:035:021  STRING " "
.../prog004_folha_pgto.cob:035:024  '.'
.../prog004_folha_pgto.cob:036:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:036:021  STRING "====00001-DISPLAY-TRABALHADOR===="
.../prog004_folha_pgto.cob:036:056  '.'
.../prog004_folha_pgto.cob:037:013  KEYWORD PERFORM
.../prog004_folha_pgto.cob:037:021  IDENTIFIER 00001-DISPLAY-TRABALHADOR
.../prog004_folha_pgto.cob:037:047  IDENTIFIER UNTIL
.../prog004_folha_pgto.cob:037:053  IDENTIFIER IDX
.../prog004_folha_pgto.cob:037:057  '>'
.../prog004_folha_pgto.cob:037:059  NUMBER 3
.../prog004_folha_pgto.cob:037:060  '.'
.../prog004_folha_pgto.cob:039:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:039:021  STRING " "
.../prog004_folha_pgto.cob:039:024  '.'
.../prog004_folha_pgto.cob:040:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:040:021  STRING "====00002-DISPLAY-BY-SEARCH===="
.../prog004_folha_pgto.cob:040:054  '.'
.../prog004_folha_pgto.cob:041:013  KEYWORD PERFORM
.../prog004_folha_pgto.cob:041:021  IDENTIFIER 00002-DISPLAY-BY-SEARCH
.../prog004_folha_pgto.cob:041:044  '.'
.../prog004_folha_pgto.cob:043:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:043:021  STRING " "
.../prog004_folha_pgto.cob:043:024  '.'
.../prog004_folha_pgto.cob:044:013  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:044:021  STRING "====00003-REMOVE-TRABALHADOR===="
.../prog004_folha_pgto.cob:044:055  '.'
.../prog004_folha_pgto.cob:045:013  KEYWORD PERFORM
.../prog004_folha_pgto.cob:045:021  IDENTIFIER 00003-REMOVE-TRABALHADOR
.../prog004_folha_pgto.cob:045:045  '.'
.../prog004_folha_pgto.cob:047:012  KEYWORD STOP
.../prog004_folha_pgto.cob:047:017  KEYWORD RUN
.../prog004_folha_pgto.cob:047:020  '.'
.../prog004_folha_pgto.cob:049:008  IDENTIFIER 00001-DISPLAY-TRABALHADOR
.../prog004_folha_pgto.cob:049:033  '.'
.../prog004_folha_pgto.cob:050:012  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:050:020  STRING " "
.../prog004_folha_pgto.cob:050:023  '.'
.../prog004_folha_pgto.cob:052:012  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:052:020  STRING "IDX="
.../prog004_folha_pgto.cob:052:027  IDENTIFIER IDX
.../prog004_folha_pgto.cob:052:030  '.'
.../prog004_folha_pgto.cob:053:012  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:053:020  STRING "TRABALHADOR(IDX)="
.../prog004_folha_pgto.cob:053:040  IDENTIFIER TRABALHADOR
.../prog004_folha_pgto.cob:053:051  '('
.../prog004_folha_pgto.cob:053:052  IDENTIFIER IDX
.../prog004_folha_pgto.cob:053:055  ')'
.../prog004_folha_pgto.cob:053:056  '.'
.../prog004_folha_pgto.cob:054:012  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:054:020  STRING "NOME(IDX)="
.../prog004_folha_pgto.cob:054:033  IDENTIFIER NOME
.../prog004_folha_pgto.cob:054:037  '('
.../prog004_folha_pgto.cob:054:038  IDENTIFIER IDX
.../prog004_folha_pgto.cob:054:041  ')'
.../prog004_folha_pgto.cob:054:042  '.'
.../prog004_folha_pgto.cob:055:012  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:055:020  STRING "SALARIO(IDX)="
.../prog004_folha_pgto.cob:055:036  IDENTIFIER SALARIO
.../prog004_folha_pgto.cob:055:043  '('
.../prog004_folha_pgto.cob:055:044  IDENTIFIER IDX
.../prog004_folha_pgto.cob:055:047  ')'
.../prog004_folha_pgto.cob:055:048  '.'
.../prog004_folha_pgto.cob:056:012  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:056:020  STRING "DPTO(IDX,1)="
.../prog004_folha_pgto.cob:056:035  IDENTIFIER DPTO
.../prog004_folha_pgto.cob:056:039  '('
.../prog004_folha_pgto.cob:056:040  IDENTIFIER IDX
.../prog004_folha_pgto.cob:056:043  ','
.../prog004_folha_pgto.cob:056:044  NUMBER 1
.../prog004_folha_pgto.cob:056:045  ')'
.../prog004_folha_pgto.cob:056:046  '.'
.../prog004_folha_pgto.cob:057:012  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:057:020  STRING "DPTO(IDX,2)="
.../prog004_folha_pgto.cob:057:035  IDENTIFIER DPTO
.../prog004_folha_pgto.cob:057:039  '('
.../prog004_folha_pgto.cob:057:040  IDENTIFIER IDX
.../prog004_folha_pgto.cob:057:043  ','
.../prog004_folha_pgto.cob:057:044  NUMBER 2
.../prog004_folha_pgto.cob:057:045  ')'
.../prog004_folha_pgto.cob:057:046  '.'
.../prog004_folha_pgto.cob:058:012  IDENTIFIER ADD
.../prog004_folha_pgto.cob:058:016  NUMBER 1
.../prog004_folha_pgto.cob:058:018  IDENTIFIER TO
.../prog004_folha_pgto.cob:058:021  IDENTIFIER IDX
.../prog004_folha_pgto.cob:058:024  '.'
.../prog004_folha_pgto.cob:060:008  IDENTIFIER 00002-DISPLAY-BY-SEARCH
.../prog004_folha_pgto.cob:060:031  '.'
.../prog004_folha_pgto.cob:062:012  IDENTIFIER SET
.../prog004_folha_pgto.cob:062:016  IDENTIFIER TID
.../prog004_folha_pgto.cob:062:020  IDENTIFIER TO
.../prog004_folha_pgto.cob:062:023  NUMBER 1
.../prog004_folha_pgto.cob:062:024  '.'
.../prog004_folha_pgto.cob:063:012  IDENTIFIER SEARCH
.../prog004_folha_pgto.cob:063:019  IDENTIFIER TRABALHADOR
.../prog004_folha_pgto.cob:064:015  IDENTIFIER AT
.../prog004_folha_pgto.cob:064:018  IDENTIFIER END
.../prog004_folha_pgto.cob:064:022  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:064:030  STRING "TRABALHADOR NOT FOUND"
.../prog004_folha_pgto.cob:065:015  IDENTIFIER WHEN
.../prog004_folha_pgto.cob:065:020  IDENTIFIER NOME
.../prog004_folha_pgto.cob:065:024  '('
.../prog004_folha_pgto.cob:065:025  IDENTIFIER TID
.../prog004_folha_pgto.cob:065:028  ')'
.../prog004_folha_pgto.cob:065:030  '='
.../prog004_folha_pgto.cob:065:032  STRING "MARIA"
.../prog004_folha_pgto.cob:066:018  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:066:026  STRING "'WHEN NOME(TID) = MARIA'"
.../prog004_folha_pgto.cob:067:018  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:067:026  STRING "TID="
.../prog004_folha_pgto.cob:067:033  IDENTIFIER TID
.../prog004_folha_pgto.cob:068:018  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:068:026  STRING "TRABALHADOR(TID)="
.../prog004_folha_pgto.cob:068:046  IDENTIFIER TRABALHADOR
.../prog004_folha_pgto.cob:068:057  '('
.../prog004_folha_pgto.cob:068:058  IDENTIFIER TID
.../prog004_folha_pgto.cob:068:061  ')'
.../prog004_folha_pgto.cob:069:012  IDENTIFIER END-SEARCH
.../prog004_folha_pgto.cob:069:022  '.'
.../prog004_folha_pgto.cob:072:008  IDENTIFIER 00003-REMOVE-TRABALHADOR
.../prog004_folha_pgto.cob:072:032  '.'
.../prog004_folha_pgto.cob:073:012  IDENTIFIER SET
.../prog004_folha_pgto.cob:073:016  IDENTIFIER TID
.../prog004_folha_pgto.cob:073:020  IDENTIFIER TO
.../prog004_folha_pgto.cob:073:023  NUMBER 2
.../prog004_folha_pgto.cob:073:024  '.'
.../prog004_folha_pgto.cob:074:012  KEYWORD MOVE
.../prog004_folha_pgto.cob:074:017  IDENTIFIER LOW-VALUES
.../prog004_folha_pgto.cob:074:028  IDENTIFIER TO
.../prog004_folha_pgto.cob:074:031  IDENTIFIER TRABALHADOR
.../prog004_folha_pgto.cob:074:042  '('
.../prog004_folha_pgto.cob:074:043  IDENTIFIER TID
.../prog004_folha_pgto.cob:074:046  ')'
.../prog004_folha_pgto.cob:075:012  KEYWORD DISPLAY
.../prog004_folha_pgto.cob:075:020  IDENTIFIER TAB-TRABALHADOR
.../prog004_folha_pgto.cob:075:035  '.'
.../prog004_folha_pgto.cob:077:008  IDENTIFIER END
.../prog004_folha_pgto.cob:077:012  IDENTIFIER PROGRAM
.../prog004_folha_pgto.cob:077:020  IDENTIFIER FOLHA-PGTO
.../prog004_folha_pgto.cob:077:030  '.'
```

=== `prog005_strings.cob`

Comando:
`cargo run -- lex cobol-examples/prog005_strings.cob`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/cobol-compiler lex .../prog005_strings.cob`
cobol::lexer::underscore_in_identifier

  ⚠ identifier 'PROG008_STRING' contains '_'
    ╭─[.../prog005_strings.cob:12:20]
 11 │        IDENTIFICATION DIVISION.
 12 │        PROGRAM-ID. PROG008_STRING.
    ·                    ───────┬──────
    ·                           ╰── underscores are not part of the specified identifier syntax
 13 │        DATA DIVISION.
    ╰────

cobol::lexer::underscore_in_identifier

  ⚠ identifier 'PROG008_STRING' contains '_'
    ╭─[.../prog005_strings.cob:75:20]
 74 │
 75 │        END PROGRAM PROG008_STRING.
    ·                    ───────┬──────
    ·                           ╰── underscores are not part of the specified identifier syntax
    ╰────

.../prog005_strings.cob:011:008  KEYWORD IDENTIFICATION
.../prog005_strings.cob:011:023  KEYWORD DIVISION
.../prog005_strings.cob:011:031  '.'
.../prog005_strings.cob:012:008  IDENTIFIER PROGRAM-ID
.../prog005_strings.cob:012:018  '.'
.../prog005_strings.cob:012:020  IDENTIFIER PROG008_STRING
.../prog005_strings.cob:012:034  '.'
.../prog005_strings.cob:013:008  KEYWORD DATA
.../prog005_strings.cob:013:013  KEYWORD DIVISION
.../prog005_strings.cob:013:021  '.'
.../prog005_strings.cob:014:008  IDENTIFIER FILE
.../prog005_strings.cob:014:013  KEYWORD SECTION
.../prog005_strings.cob:014:020  '.'
.../prog005_strings.cob:015:008  IDENTIFIER WORKING-STORAGE
.../prog005_strings.cob:015:024  KEYWORD SECTION
.../prog005_strings.cob:015:031  '.'
.../prog005_strings.cob:017:008  NUMBER 77
.../prog005_strings.cob:017:011  IDENTIFIER SAUX
.../prog005_strings.cob:017:027  PICTURE
.../prog005_strings.cob:017:031  PICTURE-STRING X(15)
.../prog005_strings.cob:017:036  '.'
.../prog005_strings.cob:018:008  NUMBER 77
.../prog005_strings.cob:018:011  IDENTIFIER CONT
.../prog005_strings.cob:018:027  PICTURE
.../prog005_strings.cob:018:031  PICTURE-STRING 9(2)
.../prog005_strings.cob:018:035  '.'
.../prog005_strings.cob:019:008  NUMBER 77
.../prog005_strings.cob:019:011  IDENTIFIER YESNO
.../prog005_strings.cob:019:027  PICTURE
.../prog005_strings.cob:019:031  PICTURE-STRING 9
.../prog005_strings.cob:019:032  '.'
.../prog005_strings.cob:021:008  NUMBER 77
.../prog005_strings.cob:021:011  IDENTIFIER IDX
.../prog005_strings.cob:021:015  PICTURE
.../prog005_strings.cob:021:019  PICTURE-STRING 9(1)
.../prog005_strings.cob:021:023  '.'
.../prog005_strings.cob:023:008  KEYWORD PROCEDURE
.../prog005_strings.cob:023:018  KEYWORD DIVISION
.../prog005_strings.cob:023:026  '.'
.../prog005_strings.cob:024:008  IDENTIFIER MAIN-PROCEDURE
.../prog005_strings.cob:024:022  '.'
.../prog005_strings.cob:025:013  KEYWORD MOVE
.../prog005_strings.cob:025:018  STRING "HELLO WORLD 123!!"
.../prog005_strings.cob:025:038  IDENTIFIER TO
.../prog005_strings.cob:025:041  IDENTIFIER SAUX
.../prog005_strings.cob:025:045  '.'
.../prog005_strings.cob:027:013  KEYWORD DISPLAY
.../prog005_strings.cob:027:021  STRING " "
.../prog005_strings.cob:027:024  '.'
.../prog005_strings.cob:028:013  KEYWORD DISPLAY
.../prog005_strings.cob:028:021  STRING "==== EXEMPLO 1 ===="
.../prog005_strings.cob:028:042  '.'
.../prog005_strings.cob:029:013  KEYWORD DISPLAY
.../prog005_strings.cob:029:021  STRING "SAUX='"
.../prog005_strings.cob:029:030  IDENTIFIER SAUX
.../prog005_strings.cob:029:035  STRING "'"
.../prog005_strings.cob:029:038  '.'
.../prog005_strings.cob:031:013  KEYWORD MOVE
.../prog005_strings.cob:031:018  NUMBER 0
.../prog005_strings.cob:031:020  IDENTIFIER TO
.../prog005_strings.cob:031:023  IDENTIFIER CONT
.../prog005_strings.cob:031:027  '.'
.../prog005_strings.cob:032:013  IDENTIFIER INSPECT
.../prog005_strings.cob:032:021  IDENTIFIER SAUX
.../prog005_strings.cob:032:026  IDENTIFIER TALLYING
.../prog005_strings.cob:032:035  IDENTIFIER CONT
.../prog005_strings.cob:032:040  IDENTIFIER FOR
.../prog005_strings.cob:032:044  IDENTIFIER CHARACTERS
.../prog005_strings.cob:032:054  '.'
.../prog005_strings.cob:033:013  KEYWORD DISPLAY
.../prog005_strings.cob:033:021  STRING "CONT="
.../prog005_strings.cob:033:029  IDENTIFIER CONT
.../prog005_strings.cob:033:034  STRING " - FOR CHARACTERS"
.../prog005_strings.cob:033:053  '.'
.../prog005_strings.cob:035:013  KEYWORD MOVE
.../prog005_strings.cob:035:018  NUMBER 0
.../prog005_strings.cob:035:020  IDENTIFIER TO
.../prog005_strings.cob:035:023  IDENTIFIER CONT
.../prog005_strings.cob:035:027  '.'
.../prog005_strings.cob:036:013  IDENTIFIER INSPECT
.../prog005_strings.cob:036:021  IDENTIFIER SAUX
.../prog005_strings.cob:036:026  IDENTIFIER TALLYING
.../prog005_strings.cob:036:035  IDENTIFIER CONT
.../prog005_strings.cob:036:040  IDENTIFIER FOR
.../prog005_strings.cob:036:044  IDENTIFIER ALL
.../prog005_strings.cob:036:048  STRING "L"
.../prog005_strings.cob:036:051  '.'
.../prog005_strings.cob:037:013  KEYWORD DISPLAY
.../prog005_strings.cob:037:021  STRING "CONT="
.../prog005_strings.cob:037:029  IDENTIFIER CONT
.../prog005_strings.cob:037:034  STRING " - FOR ALL 'L'"
.../prog005_strings.cob:037:050  '.'
.../prog005_strings.cob:039:013  KEYWORD MOVE
.../prog005_strings.cob:039:018  NUMBER 0
.../prog005_strings.cob:039:020  IDENTIFIER TO
.../prog005_strings.cob:039:023  IDENTIFIER CONT
.../prog005_strings.cob:039:027  '.'
.../prog005_strings.cob:040:013  IDENTIFIER INSPECT
.../prog005_strings.cob:040:021  IDENTIFIER SAUX
.../prog005_strings.cob:040:026  IDENTIFIER TALLYING
.../prog005_strings.cob:040:035  IDENTIFIER CONT
.../prog005_strings.cob:041:021  IDENTIFIER FOR
.../prog005_strings.cob:041:025  IDENTIFIER CHARACTERS
.../prog005_strings.cob:041:036  IDENTIFIER BEFORE
.../prog005_strings.cob:041:043  IDENTIFIER INITIAL
.../prog005_strings.cob:041:051  STRING " "
.../prog005_strings.cob:041:054  '.'
.../prog005_strings.cob:042:013  KEYWORD DISPLAY
.../prog005_strings.cob:042:021  STRING "CONT="
.../prog005_strings.cob:042:029  IDENTIFIER CONT
.../prog005_strings.cob:042:034  STRING " - BEFORE INITIAL ' '"
.../prog005_strings.cob:042:057  '.'
.../prog005_strings.cob:044:013  KEYWORD MOVE
.../prog005_strings.cob:044:018  NUMBER 0
.../prog005_strings.cob:044:020  IDENTIFIER TO
.../prog005_strings.cob:044:023  IDENTIFIER CONT
.../prog005_strings.cob:044:027  '.'
.../prog005_strings.cob:045:013  IDENTIFIER INSPECT
.../prog005_strings.cob:045:021  IDENTIFIER SAUX
.../prog005_strings.cob:045:026  IDENTIFIER TALLYING
.../prog005_strings.cob:045:035  IDENTIFIER CONT
.../prog005_strings.cob:046:021  IDENTIFIER FOR
.../prog005_strings.cob:046:025  IDENTIFIER CHARACTERS
.../prog005_strings.cob:046:036  IDENTIFIER AFTER
.../prog005_strings.cob:046:042  IDENTIFIER INITIAL
.../prog005_strings.cob:046:050  STRING " "
.../prog005_strings.cob:046:053  '.'
.../prog005_strings.cob:047:013  KEYWORD DISPLAY
.../prog005_strings.cob:047:021  STRING "CONT="
.../prog005_strings.cob:047:029  IDENTIFIER CONT
.../prog005_strings.cob:047:034  STRING " - AFTER INITIAL ' '"
.../prog005_strings.cob:047:056  '.'
.../prog005_strings.cob:049:013  KEYWORD MOVE
.../prog005_strings.cob:049:018  NUMBER 0
.../prog005_strings.cob:049:020  IDENTIFIER TO
.../prog005_strings.cob:049:023  IDENTIFIER YESNO
.../prog005_strings.cob:049:028  '.'
.../prog005_strings.cob:050:013  IDENTIFIER INSPECT
.../prog005_strings.cob:050:021  IDENTIFIER SAUX
.../prog005_strings.cob:050:026  IDENTIFIER TALLYING
.../prog005_strings.cob:050:035  IDENTIFIER YESNO
.../prog005_strings.cob:051:021  IDENTIFIER FOR
.../prog005_strings.cob:051:025  IDENTIFIER LEADING
.../prog005_strings.cob:051:033  STRING "H"
.../prog005_strings.cob:051:036  '.'
.../prog005_strings.cob:052:013  KEYWORD DISPLAY
.../prog005_strings.cob:052:021  STRING "YESNO="
.../prog005_strings.cob:052:030  IDENTIFIER YESNO
.../prog005_strings.cob:052:036  STRING " - FOR LEADING 'H' 0=N / 1=Y"
.../prog005_strings.cob:052:066  '.'
.../prog005_strings.cob:054:013  IDENTIFIER INSPECT
.../prog005_strings.cob:054:021  IDENTIFIER SAUX
.../prog005_strings.cob:054:026  IDENTIFIER REPLACING
.../prog005_strings.cob:054:036  IDENTIFIER FIRST
.../prog005_strings.cob:054:042  STRING "E"
.../prog005_strings.cob:054:046  IDENTIFIER BY
.../prog005_strings.cob:054:049  STRING "A"
.../prog005_strings.cob:054:052  '.'
.../prog005_strings.cob:055:013  KEYWORD DISPLAY
.../prog005_strings.cob:055:021  STRING "SAUX='"
.../prog005_strings.cob:055:030  IDENTIFIER SAUX
.../prog005_strings.cob:055:035  STRING "' - REPLACING FIRST 'E' BY 'A'"
.../prog005_strings.cob:055:067  '.'
.../prog005_strings.cob:057:013  IDENTIFIER INSPECT
.../prog005_strings.cob:057:021  IDENTIFIER SAUX
.../prog005_strings.cob:057:026  IDENTIFIER REPLACING
.../prog005_strings.cob:057:036  IDENTIFIER ALL
.../prog005_strings.cob:057:040  STRING "LLO"
.../prog005_strings.cob:057:046  IDENTIFIER BY
.../prog005_strings.cob:057:049  STRING "PPY"
.../prog005_strings.cob:057:054  '.'
.../prog005_strings.cob:058:013  KEYWORD DISPLAY
.../prog005_strings.cob:058:021  STRING "SAUX='"
.../prog005_strings.cob:058:030  IDENTIFIER SAUX
.../prog005_strings.cob:058:035  STRING "' - REPLACING ALL 'LLO' BY 'PPY'"
.../prog005_strings.cob:058:069  '.'
.../prog005_strings.cob:060:013  KEYWORD MOVE
.../prog005_strings.cob:060:018  STRING " HOLA-DAY"
.../prog005_strings.cob:060:030  IDENTIFIER TO
.../prog005_strings.cob:060:033  IDENTIFIER SAUX
.../prog005_strings.cob:060:037  '.'
.../prog005_strings.cob:061:013  IDENTIFIER INSPECT
.../prog005_strings.cob:061:021  IDENTIFIER SAUX
.../prog005_strings.cob:061:026  IDENTIFIER REPLACING
.../prog005_strings.cob:061:036  IDENTIFIER ALL
.../prog005_strings.cob:061:040  STRING "HOLA"
.../prog005_strings.cob:061:047  IDENTIFIER BY
.../prog005_strings.cob:061:050  STRING "GOOD"
.../prog005_strings.cob:062:021  IDENTIFIER AFTER
.../prog005_strings.cob:062:027  IDENTIFIER INITIAL
.../prog005_strings.cob:062:035  STRING " "
.../prog005_strings.cob:062:039  IDENTIFIER BEFORE
.../prog005_strings.cob:062:046  IDENTIFIER INITIAL
.../prog005_strings.cob:062:054  STRING "-"
.../prog005_strings.cob:062:057  '.'
.../prog005_strings.cob:063:013  KEYWORD DISPLAY
.../prog005_strings.cob:063:021  STRING "SAUX='"
.../prog005_strings.cob:063:030  IDENTIFIER SAUX
.../prog005_strings.cob:063:035  STRING "' - HOLA-DAY TO GOOD-DAY'"
.../prog005_strings.cob:063:062  '.'
.../prog005_strings.cob:065:013  KEYWORD MOVE
.../prog005_strings.cob:065:018  STRING "HELLO WORLD"
.../prog005_strings.cob:065:032  IDENTIFIER TO
.../prog005_strings.cob:065:035  IDENTIFIER SAUX
.../prog005_strings.cob:065:039  '.'
.../prog005_strings.cob:066:013  IDENTIFIER INSPECT
.../prog005_strings.cob:066:021  IDENTIFIER SAUX
.../prog005_strings.cob:066:026  IDENTIFIER CONVERTING
.../prog005_strings.cob:066:037  STRING "ABCDEFGHIJKLMNOPQRSTUVWXZ"
.../prog005_strings.cob:067:017  IDENTIFIER TO
.../prog005_strings.cob:067:020  STRING "abcdefghijklmnopqrstuvwxz"
.../prog005_strings.cob:067:047  '.'
.../prog005_strings.cob:068:013  KEYWORD DISPLAY
.../prog005_strings.cob:068:021  STRING "SAUX='"
.../prog005_strings.cob:068:030  IDENTIFIER SAUX
.../prog005_strings.cob:068:035  STRING "' - CONVERTING'"
.../prog005_strings.cob:068:052  '.'
.../prog005_strings.cob:070:013  IDENTIFIER INSPECT
.../prog005_strings.cob:070:021  IDENTIFIER SAUX
.../prog005_strings.cob:070:026  IDENTIFIER REPLACING
.../prog005_strings.cob:070:036  IDENTIFIER CHARACTERS
.../prog005_strings.cob:070:047  IDENTIFIER BY
.../prog005_strings.cob:070:050  STRING "F"
.../prog005_strings.cob:070:053  '.'
.../prog005_strings.cob:071:013  KEYWORD DISPLAY
.../prog005_strings.cob:071:021  STRING "SAUX='"
.../prog005_strings.cob:071:030  IDENTIFIER SAUX
.../prog005_strings.cob:071:035  STRING "' - REPLACING "
.../prog005_strings.cob:071:051  '.'
.../prog005_strings.cob:073:012  KEYWORD STOP
.../prog005_strings.cob:073:017  KEYWORD RUN
.../prog005_strings.cob:073:020  '.'
.../prog005_strings.cob:075:008  IDENTIFIER END
.../prog005_strings.cob:075:012  IDENTIFIER PROGRAM
.../prog005_strings.cob:075:020  IDENTIFIER PROG008_STRING
.../prog005_strings.cob:075:034  '.'
```

=== `prog006_files.cob`

Comando:
`cargo run -- lex cobol-examples/prog006_files.cob`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/cobol-compiler lex .../prog006_files.cob`
cobol::lexer::underscore_in_identifier

  ⚠ identifier 'PROG006_FILES' contains '_'
    ╭─[.../prog006_files.cob:12:20]
 11 │        IDENTIFICATION DIVISION.
 12 │        PROGRAM-ID. PROG006_FILES.
    ·                    ──────┬──────
    ·                          ╰── underscores are not part of the specified identifier syntax
 13 │        ENVIRONMENT DIVISION.
    ╰────

cobol::lexer::underscore_in_identifier

  ⚠ identifier 'PROG006_FILES' contains '_'
    ╭─[.../prog006_files.cob:76:20]
 75 │
 76 │        END PROGRAM PROG006_FILES.
    ·                    ──────┬──────
    ·                          ╰── underscores are not part of the specified identifier syntax
    ╰────

.../prog006_files.cob:011:008  KEYWORD IDENTIFICATION
.../prog006_files.cob:011:023  KEYWORD DIVISION
.../prog006_files.cob:011:031  '.'
.../prog006_files.cob:012:008  IDENTIFIER PROGRAM-ID
.../prog006_files.cob:012:018  '.'
.../prog006_files.cob:012:020  IDENTIFIER PROG006_FILES
.../prog006_files.cob:012:033  '.'
.../prog006_files.cob:013:008  KEYWORD ENVIRONMENT
.../prog006_files.cob:013:020  KEYWORD DIVISION
.../prog006_files.cob:013:028  '.'
.../prog006_files.cob:014:008  IDENTIFIER INPUT-OUTPUT
.../prog006_files.cob:014:021  KEYWORD SECTION
.../prog006_files.cob:014:028  '.'
.../prog006_files.cob:016:008  IDENTIFIER FILE-CONTROL
.../prog006_files.cob:016:020  '.'
.../prog006_files.cob:017:012  IDENTIFIER SELECT
.../prog006_files.cob:017:019  IDENTIFIER MY-FILE
.../prog006_files.cob:017:027  IDENTIFIER ASSIGN
.../prog006_files.cob:017:034  IDENTIFIER TO
.../prog006_files.cob:017:037  STRING "DADOS.TXT"
.../prog006_files.cob:017:048  '.'
.../prog006_files.cob:019:008  KEYWORD DATA
.../prog006_files.cob:019:013  KEYWORD DIVISION
.../prog006_files.cob:019:021  '.'
.../prog006_files.cob:020:008  IDENTIFIER FILE
.../prog006_files.cob:020:013  KEYWORD SECTION
.../prog006_files.cob:020:020  '.'
.../prog006_files.cob:023:008  IDENTIFIER FD
.../prog006_files.cob:023:011  IDENTIFIER MY-FILE
.../prog006_files.cob:023:018  '.'
.../prog006_files.cob:024:008  NUMBER 01
.../prog006_files.cob:024:011  IDENTIFIER RC-TRABALHADOR
.../prog006_files.cob:024:025  '.'
.../prog006_files.cob:025:011  NUMBER 02
.../prog006_files.cob:025:014  IDENTIFIER RC-TID
.../prog006_files.cob:025:033  PICTURE
.../prog006_files.cob:025:037  PICTURE-STRING 9(5)
.../prog006_files.cob:025:041  '.'
.../prog006_files.cob:026:011  NUMBER 02
.../prog006_files.cob:026:014  IDENTIFIER RC-NOME
.../prog006_files.cob:026:033  PICTURE
.../prog006_files.cob:026:037  PICTURE-STRING X(10)
.../prog006_files.cob:026:042  '.'
.../prog006_files.cob:027:011  NUMBER 02
.../prog006_files.cob:027:014  IDENTIFIER RC-SALARIO
.../prog006_files.cob:027:033  PICTURE
.../prog006_files.cob:027:037  PICTURE-STRING 9(10)V99
.../prog006_files.cob:027:045  '.'
.../prog006_files.cob:028:011  NUMBER 02
.../prog006_files.cob:028:014  IDENTIFIER RC-DPTO-1
.../prog006_files.cob:028:033  PICTURE
.../prog006_files.cob:028:037  PICTURE-STRING X(2)
.../prog006_files.cob:028:041  '.'
.../prog006_files.cob:029:011  NUMBER 02
.../prog006_files.cob:029:014  IDENTIFIER RC-DPTO-2
.../prog006_files.cob:029:033  PICTURE
.../prog006_files.cob:029:037  PICTURE-STRING X(2)
.../prog006_files.cob:029:041  '.'
.../prog006_files.cob:031:008  IDENTIFIER WORKING-STORAGE
.../prog006_files.cob:031:024  KEYWORD SECTION
.../prog006_files.cob:031:031  '.'
.../prog006_files.cob:033:008  NUMBER 01
.../prog006_files.cob:033:011  IDENTIFIER TAB-TRABALHADOR
.../prog006_files.cob:033:026  '.'
.../prog006_files.cob:034:011  NUMBER 02
.../prog006_files.cob:034:014  IDENTIFIER TRABALHADOR
.../prog006_files.cob:034:026  IDENTIFIER OCCURS
.../prog006_files.cob:034:033  NUMBER 3
.../prog006_files.cob:034:035  IDENTIFIER TIMES
.../prog006_files.cob:034:041  IDENTIFIER INDEXED
.../prog006_files.cob:034:049  IDENTIFIER BY
.../prog006_files.cob:034:052  IDENTIFIER TID
.../prog006_files.cob:034:055  '.'
.../prog006_files.cob:035:014  NUMBER 03
.../prog006_files.cob:035:017  IDENTIFIER NOME
.../prog006_files.cob:035:033  PICTURE
.../prog006_files.cob:035:037  PICTURE-STRING X(5)
.../prog006_files.cob:035:041  '.'
.../prog006_files.cob:036:014  NUMBER 03
.../prog006_files.cob:036:017  IDENTIFIER SALARIO
.../prog006_files.cob:036:033  PICTURE
.../prog006_files.cob:036:037  PICTURE-STRING 9(5)V99
.../prog006_files.cob:036:044  '.'
.../prog006_files.cob:037:014  NUMBER 03
.../prog006_files.cob:037:017  IDENTIFIER DPTO
.../prog006_files.cob:037:033  PICTURE
.../prog006_files.cob:037:037  PICTURE-STRING X(2)
.../prog006_files.cob:037:042  IDENTIFIER VALUES
.../prog006_files.cob:037:049  IDENTIFIER SPACES
.../prog006_files.cob:037:056  IDENTIFIER OCCURS
.../prog006_files.cob:037:063  NUMBER 2
.../prog006_files.cob:037:065  IDENTIFIER TIMES
.../prog006_files.cob:037:070  '.'
.../prog006_files.cob:039:008  NUMBER 77
.../prog006_files.cob:039:011  IDENTIFIER SAUX
.../prog006_files.cob:039:027  PICTURE
.../prog006_files.cob:039:031  PICTURE-STRING X(15)
.../prog006_files.cob:039:036  '.'
.../prog006_files.cob:040:008  NUMBER 77
.../prog006_files.cob:040:011  IDENTIFIER CONT
.../prog006_files.cob:040:027  PICTURE
.../prog006_files.cob:040:031  PICTURE-STRING 9(2)
.../prog006_files.cob:040:035  '.'
.../prog006_files.cob:041:008  NUMBER 77
.../prog006_files.cob:041:011  IDENTIFIER YESNO
.../prog006_files.cob:041:027  PICTURE
.../prog006_files.cob:041:031  PICTURE-STRING 9
.../prog006_files.cob:041:032  '.'
.../prog006_files.cob:043:008  NUMBER 77
.../prog006_files.cob:043:011  IDENTIFIER IDX
.../prog006_files.cob:043:015  PICTURE
.../prog006_files.cob:043:019  PICTURE-STRING 9(1)
.../prog006_files.cob:043:023  '.'
.../prog006_files.cob:045:008  KEYWORD PROCEDURE
.../prog006_files.cob:045:018  KEYWORD DIVISION
.../prog006_files.cob:045:026  '.'
.../prog006_files.cob:046:008  IDENTIFIER MAIN-PROCEDURE
.../prog006_files.cob:046:022  '.'
.../prog006_files.cob:048:013  KEYWORD MOVE
.../prog006_files.cob:048:018  STRING "JOSEA1111111A1A4MARIA2222222A2  FABIO3333333A2A3"
.../prog006_files.cob:048:069  IDENTIFIER TO
.../prog006_files.cob:048:072  IDENTIFIER T
.../prog006_files.cob:049:008  IDENTIFIER AB-TRABALHADOR
.../prog006_files.cob:049:022  '.'
.../prog006_files.cob:051:013  KEYWORD DISPLAY
.../prog006_files.cob:051:021  STRING " "
.../prog006_files.cob:051:024  '.'
.../prog006_files.cob:052:013  KEYWORD DISPLAY
.../prog006_files.cob:052:021  STRING "====00001-DISPLAY-TRABALHADOR===="
.../prog006_files.cob:052:056  '.'
.../prog006_files.cob:053:013  IDENTIFIER OPEN
.../prog006_files.cob:053:018  IDENTIFIER OUTPUT
.../prog006_files.cob:053:025  IDENTIFIER MY-FILE
.../prog006_files.cob:053:032  '.'
.../prog006_files.cob:055:013  KEYWORD MOVE
.../prog006_files.cob:055:018  NUMBER 1
.../prog006_files.cob:055:020  IDENTIFIER TO
.../prog006_files.cob:055:023  IDENTIFIER IDX
.../prog006_files.cob:055:026  '.'
.../prog006_files.cob:056:013  KEYWORD PERFORM
.../prog006_files.cob:056:021  IDENTIFIER 00001-WRITE-TO-FILE
.../prog006_files.cob:056:041  IDENTIFIER UNTIL
.../prog006_files.cob:056:047  IDENTIFIER IDX
.../prog006_files.cob:056:051  '>'
.../prog006_files.cob:056:053  NUMBER 3
.../prog006_files.cob:056:054  '.'
.../prog006_files.cob:058:013  IDENTIFIER CLOSE
.../prog006_files.cob:058:019  IDENTIFIER MY-FILE
.../prog006_files.cob:058:026  '.'
.../prog006_files.cob:060:012  KEYWORD STOP
.../prog006_files.cob:060:017  KEYWORD RUN
.../prog006_files.cob:060:020  '.'
.../prog006_files.cob:062:008  IDENTIFIER 00001-WRITE-TO-FILE
.../prog006_files.cob:062:027  '.'
.../prog006_files.cob:063:012  KEYWORD DISPLAY
.../prog006_files.cob:063:020  STRING " "
.../prog006_files.cob:063:023  '.'
.../prog006_files.cob:065:012  KEYWORD MOVE
.../prog006_files.cob:065:017  IDENTIFIER LOW-VALUES
.../prog006_files.cob:065:028  IDENTIFIER TO
.../prog006_files.cob:065:031  IDENTIFIER RC-TRABALHADOR
.../prog006_files.cob:065:045  '.'
.../prog006_files.cob:066:012  KEYWORD DISPLAY
.../prog006_files.cob:066:020  STRING "IDX="
.../prog006_files.cob:066:027  IDENTIFIER IDX
.../prog006_files.cob:066:030  '.'
.../prog006_files.cob:067:012  KEYWORD DISPLAY
.../prog006_files.cob:067:020  STRING "TRABALHADOR(IDX)="
.../prog006_files.cob:067:040  IDENTIFIER TRABALHADOR
.../prog006_files.cob:067:051  '('
.../prog006_files.cob:067:052  IDENTIFIER IDX
.../prog006_files.cob:067:055  ')'
.../prog006_files.cob:067:056  '.'
.../prog006_files.cob:068:012  KEYWORD MOVE
.../prog006_files.cob:068:017  IDENTIFIER IDX
.../prog006_files.cob:068:021  IDENTIFIER TO
.../prog006_files.cob:068:024  IDENTIFIER RC-TID
.../prog006_files.cob:068:030  '.'
.../prog006_files.cob:069:012  KEYWORD MOVE
.../prog006_files.cob:069:017  IDENTIFIER NOME
.../prog006_files.cob:069:021  '('
.../prog006_files.cob:069:022  IDENTIFIER IDX
.../prog006_files.cob:069:025  ')'
.../prog006_files.cob:069:027  IDENTIFIER TO
.../prog006_files.cob:069:030  IDENTIFIER RC-NOME
.../prog006_files.cob:069:037  '.'
.../prog006_files.cob:070:012  KEYWORD MOVE
.../prog006_files.cob:070:017  IDENTIFIER SALARIO
.../prog006_files.cob:070:024  '('
.../prog006_files.cob:070:025  IDENTIFIER IDX
.../prog006_files.cob:070:028  ')'
.../prog006_files.cob:070:030  IDENTIFIER TO
.../prog006_files.cob:070:033  IDENTIFIER RC-SALARIO
.../prog006_files.cob:070:043  '.'
.../prog006_files.cob:071:012  KEYWORD MOVE
.../prog006_files.cob:071:017  IDENTIFIER DPTO
.../prog006_files.cob:071:021  '('
.../prog006_files.cob:071:022  IDENTIFIER IDX
.../prog006_files.cob:071:025  ','
.../prog006_files.cob:071:026  NUMBER 1
.../prog006_files.cob:071:027  ')'
.../prog006_files.cob:071:029  IDENTIFIER TO
.../prog006_files.cob:071:032  IDENTIFIER RC-DPTO-1
.../prog006_files.cob:071:041  '.'
.../prog006_files.cob:072:012  KEYWORD MOVE
.../prog006_files.cob:072:017  IDENTIFIER DPTO
.../prog006_files.cob:072:021  '('
.../prog006_files.cob:072:022  IDENTIFIER IDX
.../prog006_files.cob:072:025  ','
.../prog006_files.cob:072:026  NUMBER 2
.../prog006_files.cob:072:027  ')'
.../prog006_files.cob:072:029  IDENTIFIER TO
.../prog006_files.cob:072:032  IDENTIFIER RC-DPTO-2
.../prog006_files.cob:072:041  '.'
.../prog006_files.cob:073:012  IDENTIFIER WRITE
.../prog006_files.cob:073:018  IDENTIFIER RC-TRABALHADOR
.../prog006_files.cob:073:032  '.'
.../prog006_files.cob:074:012  IDENTIFIER ADD
.../prog006_files.cob:074:016  NUMBER 1
.../prog006_files.cob:074:018  IDENTIFIER TO
.../prog006_files.cob:074:021  IDENTIFIER IDX
.../prog006_files.cob:074:024  '.'
.../prog006_files.cob:076:008  IDENTIFIER END
.../prog006_files.cob:076:012  IDENTIFIER PROGRAM
.../prog006_files.cob:076:020  IDENTIFIER PROG006_FILES
.../prog006_files.cob:076:033  '.'
```

=== `prog007_loja.cob`

Comando:
`cargo run -- lex cobol-examples/prog007_loja.cob`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/cobol-compiler lex .../prog007_loja.cob`
cobol::lexer::underscore_in_identifier

  ⚠ identifier 'PROG007_LOJA' contains '_'
    ╭─[.../prog007_loja.cob:12:20]
 11 │        IDENTIFICATION DIVISION.
 12 │        PROGRAM-ID. PROG007_LOJA.
    ·                    ──────┬─────
    ·                          ╰── underscores are not part of the specified identifier syntax
 13 │
    ╰────

cobol::lexer::underscore_in_identifier

  ⚠ identifier 'PROG007_LOJA' contains '_'
     ╭─[.../prog007_loja.cob:228:20]
 227 │
 228 │        END PROGRAM PROG007_LOJA.
     ·                    ──────┬─────
     ·                          ╰── underscores are not part of the specified identifier syntax
     ╰────

.../prog007_loja.cob:011:008  KEYWORD IDENTIFICATION
.../prog007_loja.cob:011:023  KEYWORD DIVISION
.../prog007_loja.cob:011:031  '.'
.../prog007_loja.cob:012:008  IDENTIFIER PROGRAM-ID
.../prog007_loja.cob:012:018  '.'
.../prog007_loja.cob:012:020  IDENTIFIER PROG007_LOJA
.../prog007_loja.cob:012:032  '.'
.../prog007_loja.cob:014:008  KEYWORD ENVIRONMENT
.../prog007_loja.cob:014:020  KEYWORD DIVISION
.../prog007_loja.cob:014:028  '.'
.../prog007_loja.cob:015:008  IDENTIFIER INPUT-OUTPUT
.../prog007_loja.cob:015:021  KEYWORD SECTION
.../prog007_loja.cob:015:028  '.'
.../prog007_loja.cob:017:008  IDENTIFIER FILE-CONTROL
.../prog007_loja.cob:017:020  '.'
.../prog007_loja.cob:018:012  IDENTIFIER SELECT
.../prog007_loja.cob:018:019  IDENTIFIER FD-PRODUTOS
.../prog007_loja.cob:018:031  IDENTIFIER ASSIGN
.../prog007_loja.cob:018:038  IDENTIFIER TO
.../prog007_loja.cob:018:041  IDENTIFIER WS-PROD-FILENAME
.../prog007_loja.cob:019:031  IDENTIFIER ORGANIZATION
.../prog007_loja.cob:019:044  IDENTIFIER INDEXED
.../prog007_loja.cob:020:031  IDENTIFIER ACCESS
.../prog007_loja.cob:020:038  IDENTIFIER MODE
.../prog007_loja.cob:020:043  IDENTIFIER DYNAMIC
.../prog007_loja.cob:021:031  IDENTIFIER RECORD
.../prog007_loja.cob:021:038  IDENTIFIER KEY
.../prog007_loja.cob:021:042  IDENTIFIER IS
.../prog007_loja.cob:021:045  IDENTIFIER RP-PID
.../prog007_loja.cob:022:031  IDENTIFIER ALTERNATE
.../prog007_loja.cob:022:041  IDENTIFIER RECORD
.../prog007_loja.cob:022:048  IDENTIFIER KEY
.../prog007_loja.cob:022:052  IDENTIFIER IS
.../prog007_loja.cob:022:055  IDENTIFIER RP-NOME
.../prog007_loja.cob:023:031  IDENTIFIER FILE
.../prog007_loja.cob:023:036  IDENTIFIER STATUS
.../prog007_loja.cob:023:043  IDENTIFIER WS-FILE-STATUS-PROD
.../prog007_loja.cob:023:062  '.'
.../prog007_loja.cob:026:012  IDENTIFIER SELECT
.../prog007_loja.cob:026:019  IDENTIFIER FD-PRODUTOS-KEY
.../prog007_loja.cob:026:035  IDENTIFIER ASSIGN
.../prog007_loja.cob:026:042  IDENTIFIER TO
.../prog007_loja.cob:026:045  STRING "PROG10-001.KEY"
.../prog007_loja.cob:027:031  IDENTIFIER FILE
.../prog007_loja.cob:027:036  IDENTIFIER STATUS
.../prog007_loja.cob:027:043  IDENTIFIER WS-FILE-STATUS-PROD-KEY
.../prog007_loja.cob:027:066  '.'
.../prog007_loja.cob:029:008  KEYWORD DATA
.../prog007_loja.cob:029:013  KEYWORD DIVISION
.../prog007_loja.cob:029:021  '.'
.../prog007_loja.cob:030:008  IDENTIFIER FILE
.../prog007_loja.cob:030:013  KEYWORD SECTION
.../prog007_loja.cob:030:020  '.'
.../prog007_loja.cob:033:008  IDENTIFIER FD
.../prog007_loja.cob:033:011  IDENTIFIER FD-PRODUTOS
.../prog007_loja.cob:034:011  IDENTIFIER BLOCK
.../prog007_loja.cob:034:017  IDENTIFIER CONTAINS
.../prog007_loja.cob:034:026  NUMBER 874
.../prog007_loja.cob:034:030  IDENTIFIER RECORDS
.../prog007_loja.cob:035:011  IDENTIFIER RECORD
.../prog007_loja.cob:035:018  IDENTIFIER CONTAINS
.../prog007_loja.cob:035:027  NUMBER 64
.../prog007_loja.cob:035:030  IDENTIFIER CHARACTERS
.../prog007_loja.cob:035:040  '.'
.../prog007_loja.cob:046:008  NUMBER 01
.../prog007_loja.cob:046:011  IDENTIFIER REG-PRODUTO
.../prog007_loja.cob:046:022  '.'
.../prog007_loja.cob:047:012  NUMBER 02
.../prog007_loja.cob:047:015  IDENTIFIER RP-PID
.../prog007_loja.cob:047:034  PICTURE
.../prog007_loja.cob:047:038  PICTURE-STRING 9(10)
.../prog007_loja.cob:047:043  '.'
.../prog007_loja.cob:048:012  NUMBER 02
.../prog007_loja.cob:048:015  IDENTIFIER RP-NOME
.../prog007_loja.cob:048:034  PICTURE
.../prog007_loja.cob:048:038  PICTURE-STRING X(10)
.../prog007_loja.cob:048:043  '.'
.../prog007_loja.cob:049:012  NUMBER 02
.../prog007_loja.cob:049:015  IDENTIFIER RP-VALOR
.../prog007_loja.cob:049:034  PICTURE
.../prog007_loja.cob:049:038  PICTURE-STRING 9(10)V9(02)
.../prog007_loja.cob:049:049  '.'
.../prog007_loja.cob:050:012  NUMBER 02
.../prog007_loja.cob:050:015  IDENTIFIER RP-QTD
.../prog007_loja.cob:050:034  PICTURE
.../prog007_loja.cob:050:038  PICTURE-STRING 9(10)V9(02)
.../prog007_loja.cob:050:049  '.'
.../prog007_loja.cob:051:012  NUMBER 02
.../prog007_loja.cob:051:015  IDENTIFIER RP-DESCRICAO
.../prog007_loja.cob:051:034  PICTURE
.../prog007_loja.cob:051:038  PICTURE-STRING X(20)
.../prog007_loja.cob:051:043  '.'
.../prog007_loja.cob:054:008  IDENTIFIER FD
.../prog007_loja.cob:054:011  IDENTIFIER FD-PRODUTOS-KEY
.../prog007_loja.cob:054:026  '.'
.../prog007_loja.cob:055:008  NUMBER 01
.../prog007_loja.cob:055:011  IDENTIFIER REGISTRO-CHAVE
.../prog007_loja.cob:055:033  PICTURE
.../prog007_loja.cob:055:037  PICTURE-STRING 9(10)
.../prog007_loja.cob:055:042  '.'
.../prog007_loja.cob:059:008  IDENTIFIER WORKING-STORAGE
.../prog007_loja.cob:059:024  KEYWORD SECTION
.../prog007_loja.cob:059:031  '.'
.../prog007_loja.cob:062:008  NUMBER 01
.../prog007_loja.cob:062:011  IDENTIFIER WS-PROD-FILENAME
.../prog007_loja.cob:062:027  '.'
.../prog007_loja.cob:063:012  NUMBER 02
.../prog007_loja.cob:063:015  IDENTIFIER NAME
.../prog007_loja.cob:063:034  PICTURE
.../prog007_loja.cob:063:038  PICTURE-STRING X(7)
.../prog007_loja.cob:063:043  IDENTIFIER VALUES
.../prog007_loja.cob:063:050  STRING "PROG10-"
.../prog007_loja.cob:063:059  '.'
.../prog007_loja.cob:064:012  NUMBER 02
.../prog007_loja.cob:064:015  IDENTIFIER NUM
.../prog007_loja.cob:064:034  PICTURE
.../prog007_loja.cob:064:038  PICTURE-STRING 9(3)
.../prog007_loja.cob:064:043  IDENTIFIER VALUES
.../prog007_loja.cob:064:050  NUMBER 1
.../prog007_loja.cob:064:051  '.'
.../prog007_loja.cob:065:012  NUMBER 02
.../prog007_loja.cob:065:015  IDENTIFIER EXT
.../prog007_loja.cob:065:034  PICTURE
.../prog007_loja.cob:065:038  PICTURE-STRING X(4)
.../prog007_loja.cob:065:043  IDENTIFIER VALUES
.../prog007_loja.cob:065:050  STRING ".DAT"
.../prog007_loja.cob:065:056  '.'
.../prog007_loja.cob:067:008  NUMBER 77
.../prog007_loja.cob:067:011  IDENTIFIER WS-FILE-STATUS-PROD
.../prog007_loja.cob:067:037  PICTURE
.../prog007_loja.cob:067:041  PICTURE-STRING 9(02)
.../prog007_loja.cob:067:046  '.'
.../prog007_loja.cob:068:008  NUMBER 77
.../prog007_loja.cob:068:011  IDENTIFIER WS-FILE-STATUS-PROD-KEY
.../prog007_loja.cob:068:037  PICTURE
.../prog007_loja.cob:068:041  PICTURE-STRING 9(02)
.../prog007_loja.cob:068:046  '.'
.../prog007_loja.cob:070:008  NUMBER 01
.../prog007_loja.cob:070:011  IDENTIFIER WS-PRODUTO
.../prog007_loja.cob:070:021  '.'
.../prog007_loja.cob:071:012  NUMBER 02
.../prog007_loja.cob:071:015  IDENTIFIER WS-PID
.../prog007_loja.cob:071:034  PICTURE
.../prog007_loja.cob:071:038  PICTURE-STRING 9(10)
.../prog007_loja.cob:071:043  '.'
.../prog007_loja.cob:072:012  NUMBER 02
.../prog007_loja.cob:072:015  IDENTIFIER WS-NOME
.../prog007_loja.cob:072:034  PICTURE
.../prog007_loja.cob:072:038  PICTURE-STRING X(10)
.../prog007_loja.cob:072:043  '.'
.../prog007_loja.cob:073:012  NUMBER 02
.../prog007_loja.cob:073:015  IDENTIFIER WS-VALOR
.../prog007_loja.cob:073:034  PICTURE
.../prog007_loja.cob:073:038  PICTURE-STRING 9(10)V9(02)
.../prog007_loja.cob:073:049  '.'
.../prog007_loja.cob:074:012  NUMBER 02
.../prog007_loja.cob:074:015  IDENTIFIER WS-QTD
.../prog007_loja.cob:074:034  PICTURE
.../prog007_loja.cob:074:038  PICTURE-STRING 9(10)V9(02)
.../prog007_loja.cob:074:049  '.'
.../prog007_loja.cob:075:012  NUMBER 02
.../prog007_loja.cob:075:015  IDENTIFIER WS-DESCRICAO
.../prog007_loja.cob:075:034  PICTURE
.../prog007_loja.cob:075:038  PICTURE-STRING X(20)
.../prog007_loja.cob:075:043  '.'
.../prog007_loja.cob:078:008  NUMBER 77
.../prog007_loja.cob:078:011  IDENTIFIER WS-OPCAO
.../prog007_loja.cob:078:043  PICTURE
.../prog007_loja.cob:078:047  PICTURE-STRING 9(01)
.../prog007_loja.cob:078:052  '.'
.../prog007_loja.cob:079:008  NUMBER 77
.../prog007_loja.cob:079:011  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:079:043  PICTURE
.../prog007_loja.cob:079:047  PICTURE-STRING 9(10)
.../prog007_loja.cob:079:052  '.'
.../prog007_loja.cob:080:008  NUMBER 77
.../prog007_loja.cob:080:011  IDENTIFIER WS-EXIT
.../prog007_loja.cob:080:043  PICTURE
.../prog007_loja.cob:080:047  PICTURE-STRING 9(01)
.../prog007_loja.cob:080:053  IDENTIFIER VALUES
.../prog007_loja.cob:080:060  IDENTIFIER ZERO
.../prog007_loja.cob:080:064  '.'
.../prog007_loja.cob:084:008  KEYWORD PROCEDURE
.../prog007_loja.cob:084:018  KEYWORD DIVISION
.../prog007_loja.cob:084:026  '.'
.../prog007_loja.cob:085:008  IDENTIFIER MAIN-PROCEDURE
.../prog007_loja.cob:085:022  '.'
.../prog007_loja.cob:087:012  KEYWORD DISPLAY
.../prog007_loja.cob:087:020  STRING "PERFORM 00000-OPEN-FILE-PRODUTOS-KEY"
.../prog007_loja.cob:087:058  '.'
.../prog007_loja.cob:088:012  KEYWORD PERFORM
.../prog007_loja.cob:088:020  IDENTIFIER 00000-OPEN-FILE-PRODUTOS-KEY
.../prog007_loja.cob:088:048  '.'
.../prog007_loja.cob:090:012  KEYWORD IF
.../prog007_loja.cob:090:015  IDENTIFIER WS-EXIT
.../prog007_loja.cob:090:023  '='
.../prog007_loja.cob:090:025  NUMBER 0
.../prog007_loja.cob:091:015  KEYWORD DISPLAY
.../prog007_loja.cob:091:023  STRING "PERFORM 00000-OPEN-FILE-PRODUTOS"
.../prog007_loja.cob:092:015  KEYWORD PERFORM
.../prog007_loja.cob:092:023  IDENTIFIER 00000-OPEN-FILE-PRODUTOS
.../prog007_loja.cob:093:012  IDENTIFIER END-IF
.../prog007_loja.cob:093:018  '.'
.../prog007_loja.cob:095:012  KEYWORD PERFORM
.../prog007_loja.cob:095:020  IDENTIFIER 00001-MENU-OPCOES
.../prog007_loja.cob:095:038  IDENTIFIER UNTIL
.../prog007_loja.cob:095:044  IDENTIFIER WS-EXIT
.../prog007_loja.cob:095:052  '='
.../prog007_loja.cob:095:054  NUMBER 1
.../prog007_loja.cob:095:055  '.'
.../prog007_loja.cob:097:012  IDENTIFIER WRITE
.../prog007_loja.cob:097:018  IDENTIFIER REGISTRO-CHAVE
.../prog007_loja.cob:097:033  IDENTIFIER FROM
.../prog007_loja.cob:097:038  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:097:051  '.'
.../prog007_loja.cob:098:012  IDENTIFIER CLOSE
.../prog007_loja.cob:098:018  IDENTIFIER FD-PRODUTOS-KEY
.../prog007_loja.cob:098:033  '.'
.../prog007_loja.cob:099:012  IDENTIFIER CLOSE
.../prog007_loja.cob:099:018  IDENTIFIER FD-PRODUTOS
.../prog007_loja.cob:099:029  '.'
.../prog007_loja.cob:100:012  KEYWORD DISPLAY
.../prog007_loja.cob:100:020  STRING "BYE"
.../prog007_loja.cob:101:012  KEYWORD STOP
.../prog007_loja.cob:101:017  KEYWORD RUN
.../prog007_loja.cob:101:020  '.'
.../prog007_loja.cob:103:008  IDENTIFIER 00000-OPEN-FILE-PRODUTOS-KEY
.../prog007_loja.cob:103:036  '.'
.../prog007_loja.cob:105:012  KEYWORD MOVE
.../prog007_loja.cob:105:017  NUMBER 0
.../prog007_loja.cob:105:019  IDENTIFIER TO
.../prog007_loja.cob:105:022  IDENTIFIER WS-FILE-STATUS-PROD-KEY
.../prog007_loja.cob:105:045  '.'
.../prog007_loja.cob:106:012  IDENTIFIER OPEN
.../prog007_loja.cob:106:017  IDENTIFIER INPUT
.../prog007_loja.cob:106:023  IDENTIFIER FD-PRODUTOS-KEY
.../prog007_loja.cob:106:038  '.'
.../prog007_loja.cob:107:012  KEYWORD IF
.../prog007_loja.cob:107:015  IDENTIFIER WS-FILE-STATUS-PROD-KEY
.../prog007_loja.cob:107:039  '='
.../prog007_loja.cob:107:041  NUMBER 0
.../prog007_loja.cob:108:015  KEYWORD DISPLAY
.../prog007_loja.cob:108:023  STRING "-- PROG10-001.KEY FOUND"
.../prog007_loja.cob:109:015  IDENTIFIER READ
.../prog007_loja.cob:109:020  IDENTIFIER FD-PRODUTOS-KEY
.../prog007_loja.cob:109:036  IDENTIFIER INTO
.../prog007_loja.cob:109:041  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:110:012  KEYWORD ELSE
.../prog007_loja.cob:111:012  KEYWORD IF
.../prog007_loja.cob:111:015  IDENTIFIER WS-FILE-STATUS-PROD-KEY
.../prog007_loja.cob:111:039  '='
.../prog007_loja.cob:111:041  NUMBER 35
.../prog007_loja.cob:112:015  KEYWORD DISPLAY
.../prog007_loja.cob:112:023  STRING "-- PROG10-001.KEY NOT FOUND - CREATING NEW"
.../prog007_loja.cob:113:015  IDENTIFIER OPEN
.../prog007_loja.cob:113:020  IDENTIFIER OUTPUT
.../prog007_loja.cob:113:027  IDENTIFIER FD-PRODUTOS-KEY
.../prog007_loja.cob:114:015  KEYWORD IF
.../prog007_loja.cob:114:018  IDENTIFIER WS-FILE-STATUS-PROD-KEY
.../prog007_loja.cob:114:042  '='
.../prog007_loja.cob:114:044  NUMBER 0
.../prog007_loja.cob:115:018  KEYWORD MOVE
.../prog007_loja.cob:115:023  NUMBER 0
.../prog007_loja.cob:115:025  IDENTIFIER TO
.../prog007_loja.cob:115:028  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:116:015  KEYWORD ELSE
.../prog007_loja.cob:117:018  KEYWORD DISPLAY
.../prog007_loja.cob:117:026  STRING "-- ERRO AO CRIAR ARQUIVO DE CHAVES -->"
.../prog007_loja.cob:118:026  IDENTIFIER WS-FILE-STATUS-PROD-KEY
.../prog007_loja.cob:119:018  KEYWORD MOVE
.../prog007_loja.cob:119:023  NUMBER 1
.../prog007_loja.cob:119:025  IDENTIFIER TO
.../prog007_loja.cob:119:028  IDENTIFIER WS-EXIT
.../prog007_loja.cob:120:012  KEYWORD ELSE
.../prog007_loja.cob:121:015  KEYWORD DISPLAY
.../prog007_loja.cob:121:023  STRING "-- ERRO AO ABRIR ARQUIVO DE CHAVES -->"
.../prog007_loja.cob:122:021  IDENTIFIER WS-FILE-STATUS-PROD-KEY
.../prog007_loja.cob:123:015  KEYWORD MOVE
.../prog007_loja.cob:123:020  NUMBER 1
.../prog007_loja.cob:123:022  IDENTIFIER TO
.../prog007_loja.cob:123:025  IDENTIFIER WS-EXIT
.../prog007_loja.cob:123:032  '.'
.../prog007_loja.cob:125:008  IDENTIFIER 00000-OPEN-FILE-PRODUTOS
.../prog007_loja.cob:125:032  '.'
.../prog007_loja.cob:126:012  KEYWORD MOVE
.../prog007_loja.cob:126:017  NUMBER 0
.../prog007_loja.cob:126:019  IDENTIFIER TO
.../prog007_loja.cob:126:022  IDENTIFIER WS-FILE-STATUS-PROD
.../prog007_loja.cob:126:041  '.'
.../prog007_loja.cob:127:012  IDENTIFIER OPEN
.../prog007_loja.cob:127:017  IDENTIFIER I-O
.../prog007_loja.cob:127:021  IDENTIFIER FD-PRODUTOS
.../prog007_loja.cob:127:032  '.'
.../prog007_loja.cob:129:012  KEYWORD IF
.../prog007_loja.cob:129:015  IDENTIFIER WS-FILE-STATUS-PROD
.../prog007_loja.cob:129:035  '='
.../prog007_loja.cob:129:037  NUMBER 35
.../prog007_loja.cob:130:015  KEYWORD DISPLAY
.../prog007_loja.cob:130:023  STRING "-- PROG10-001.DAT NOT FOUND - CREATING NEW"
.../prog007_loja.cob:131:015  IDENTIFIER OPEN
.../prog007_loja.cob:131:020  IDENTIFIER OUTPUT
.../prog007_loja.cob:131:027  IDENTIFIER FD-PRODUTOS
.../prog007_loja.cob:132:015  KEYWORD IF
.../prog007_loja.cob:132:018  IDENTIFIER WS-FILE-STATUS-PROD
.../prog007_loja.cob:132:038  '='
.../prog007_loja.cob:132:040  NUMBER 0
.../prog007_loja.cob:133:018  KEYWORD MOVE
.../prog007_loja.cob:133:023  NUMBER 0
.../prog007_loja.cob:133:025  IDENTIFIER TO
.../prog007_loja.cob:133:028  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:134:015  KEYWORD ELSE
.../prog007_loja.cob:135:018  KEYWORD DISPLAY
.../prog007_loja.cob:135:026  STRING "-- ERRO AO CRIAR ARQUIVO DE DADOS -->"
.../prog007_loja.cob:136:026  IDENTIFIER WS-FILE-STATUS-PROD
.../prog007_loja.cob:137:018  KEYWORD MOVE
.../prog007_loja.cob:137:023  NUMBER 1
.../prog007_loja.cob:137:025  IDENTIFIER TO
.../prog007_loja.cob:137:028  IDENTIFIER WS-EXIT
.../prog007_loja.cob:138:012  KEYWORD ELSE
.../prog007_loja.cob:139:012  KEYWORD IF
.../prog007_loja.cob:139:015  IDENTIFIER WS-FILE-STATUS-PROD
.../prog007_loja.cob:139:035  '='
.../prog007_loja.cob:139:037  NUMBER 0
.../prog007_loja.cob:140:015  KEYWORD DISPLAY
.../prog007_loja.cob:140:023  STRING "-- PROG10-001.DAT FOUND"
.../prog007_loja.cob:141:015  KEYWORD IF
.../prog007_loja.cob:141:018  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:141:032  '='
.../prog007_loja.cob:141:034  NUMBER 0
.../prog007_loja.cob:142:018  KEYWORD MOVE
.../prog007_loja.cob:142:023  NUMBER 0
.../prog007_loja.cob:142:025  IDENTIFIER TO
.../prog007_loja.cob:142:028  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:143:015  KEYWORD ELSE
.../prog007_loja.cob:144:018  KEYWORD MOVE
.../prog007_loja.cob:144:023  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:144:037  IDENTIFIER TO
.../prog007_loja.cob:144:040  IDENTIFIER RP-PID
.../prog007_loja.cob:146:018  IDENTIFIER START
.../prog007_loja.cob:146:024  IDENTIFIER FD-PRODUTOS
.../prog007_loja.cob:146:036  IDENTIFIER KEY
.../prog007_loja.cob:146:040  IDENTIFIER EQUAL
.../prog007_loja.cob:146:046  IDENTIFIER RP-PID
.../prog007_loja.cob:147:022  IDENTIFIER INVALID
.../prog007_loja.cob:147:030  IDENTIFIER KEY
.../prog007_loja.cob:148:025  KEYWORD DISPLAY
.../prog007_loja.cob:148:033  STRING "ERRO AO REGISTRO"
.../prog007_loja.cob:149:012  KEYWORD ELSE
.../prog007_loja.cob:150:014  KEYWORD DISPLAY
.../prog007_loja.cob:150:022  STRING "-- ERRO AO ABRIR ARQUIVO DE PRODUTOS"
.../prog007_loja.cob:151:026  IDENTIFIER WS-FILE-STATUS-PROD
.../prog007_loja.cob:152:014  KEYWORD MOVE
.../prog007_loja.cob:152:019  NUMBER 1
.../prog007_loja.cob:152:021  IDENTIFIER TO
.../prog007_loja.cob:152:024  IDENTIFIER WS-EXIT
.../prog007_loja.cob:152:031  '.'
.../prog007_loja.cob:155:008  IDENTIFIER 00001-MENU-OPCOES
.../prog007_loja.cob:155:025  '.'
.../prog007_loja.cob:156:012  KEYWORD DISPLAY
.../prog007_loja.cob:156:020  STRING " "
.../prog007_loja.cob:156:023  '.'
.../prog007_loja.cob:157:012  KEYWORD DISPLAY
.../prog007_loja.cob:157:020  STRING "====00001-MENU-OPCOES===="
.../prog007_loja.cob:157:047  '.'
.../prog007_loja.cob:158:012  KEYWORD DISPLAY
.../prog007_loja.cob:158:020  STRING "OPCOES:"
.../prog007_loja.cob:158:029  '.'
.../prog007_loja.cob:159:012  KEYWORD DISPLAY
.../prog007_loja.cob:159:020  STRING "1) ADICIONA PRODUTO"
.../prog007_loja.cob:159:041  '.'
.../prog007_loja.cob:160:012  KEYWORD DISPLAY
.../prog007_loja.cob:160:020  STRING "2) REMOVE PRODUTO - NOT IMPLEMENTED"
.../prog007_loja.cob:160:057  '.'
.../prog007_loja.cob:161:012  KEYWORD DISPLAY
.../prog007_loja.cob:161:020  STRING "3) ATUALIZA PRODUTO - NOT IMPLEMENTED"
.../prog007_loja.cob:161:059  '.'
.../prog007_loja.cob:162:012  KEYWORD DISPLAY
.../prog007_loja.cob:162:020  STRING "4) CONSULTA PRODUTO"
.../prog007_loja.cob:162:041  '.'
.../prog007_loja.cob:163:012  KEYWORD DISPLAY
.../prog007_loja.cob:163:020  STRING " "
.../prog007_loja.cob:163:023  '.'
.../prog007_loja.cob:164:012  KEYWORD DISPLAY
.../prog007_loja.cob:164:020  STRING "5) SAIR DO PROGRAMA"
.../prog007_loja.cob:164:041  '.'
.../prog007_loja.cob:165:012  KEYWORD ACCEPT
.../prog007_loja.cob:165:020  IDENTIFIER WS-OPCAO
.../prog007_loja.cob:165:028  '.'
.../prog007_loja.cob:167:012  IDENTIFIER EVALUATE
.../prog007_loja.cob:167:021  IDENTIFIER WS-OPCAO
.../prog007_loja.cob:168:015  IDENTIFIER WHEN
.../prog007_loja.cob:168:020  NUMBER 1
.../prog007_loja.cob:168:022  KEYWORD PERFORM
.../prog007_loja.cob:168:030  IDENTIFIER 00002-CREATE-PRODUTO
.../prog007_loja.cob:169:015  IDENTIFIER WHEN
.../prog007_loja.cob:169:020  NUMBER 2
.../prog007_loja.cob:170:018  KEYWORD DISPLAY
.../prog007_loja.cob:170:026  STRING "DELETE"
.../prog007_loja.cob:171:015  IDENTIFIER WHEN
.../prog007_loja.cob:171:020  NUMBER 3
.../prog007_loja.cob:172:018  KEYWORD DISPLAY
.../prog007_loja.cob:172:026  STRING "UPDATE"
.../prog007_loja.cob:173:015  IDENTIFIER WHEN
.../prog007_loja.cob:173:020  NUMBER 4
.../prog007_loja.cob:173:022  KEYWORD PERFORM
.../prog007_loja.cob:173:030  IDENTIFIER 00002-SELECT-PRODUTO
.../prog007_loja.cob:174:015  IDENTIFIER WHEN
.../prog007_loja.cob:174:020  NUMBER 5
.../prog007_loja.cob:175:018  KEYWORD MOVE
.../prog007_loja.cob:175:023  NUMBER 1
.../prog007_loja.cob:175:025  IDENTIFIER TO
.../prog007_loja.cob:175:028  IDENTIFIER WS-EXIT
.../prog007_loja.cob:176:015  IDENTIFIER WHEN
.../prog007_loja.cob:176:020  IDENTIFIER OTHER
.../prog007_loja.cob:177:018  KEYWORD DISPLAY
.../prog007_loja.cob:177:026  STRING "OPCAO INVALIDA."
.../prog007_loja.cob:178:012  IDENTIFIER END-EVALUATE
.../prog007_loja.cob:178:024  '.'
.../prog007_loja.cob:180:008  IDENTIFIER 00002-CREATE-PRODUTO
.../prog007_loja.cob:180:028  '.'
.../prog007_loja.cob:181:012  KEYWORD DISPLAY
.../prog007_loja.cob:181:020  STRING "CRIA UM NOVO PRODUTO"
.../prog007_loja.cob:181:042  '.'
.../prog007_loja.cob:182:012  KEYWORD DISPLAY
.../prog007_loja.cob:182:020  STRING "NOME: "
.../prog007_loja.cob:182:029  IDENTIFIER WITH
.../prog007_loja.cob:182:034  IDENTIFIER NO
.../prog007_loja.cob:182:037  IDENTIFIER ADVANCING
.../prog007_loja.cob:182:046  '.'
.../prog007_loja.cob:183:012  KEYWORD ACCEPT
.../prog007_loja.cob:183:019  IDENTIFIER WS-NOME
.../prog007_loja.cob:183:026  '.'
.../prog007_loja.cob:184:012  KEYWORD DISPLAY
.../prog007_loja.cob:184:020  STRING "VALOR: "
.../prog007_loja.cob:184:030  IDENTIFIER WITH
.../prog007_loja.cob:184:035  IDENTIFIER NO
.../prog007_loja.cob:184:038  IDENTIFIER ADVANCING
.../prog007_loja.cob:184:047  '.'
.../prog007_loja.cob:185:012  KEYWORD ACCEPT
.../prog007_loja.cob:185:019  IDENTIFIER WS-VALOR
.../prog007_loja.cob:185:027  '.'
.../prog007_loja.cob:186:012  KEYWORD DISPLAY
.../prog007_loja.cob:186:020  STRING "QUANTIDADE: "
.../prog007_loja.cob:186:035  IDENTIFIER WITH
.../prog007_loja.cob:186:040  IDENTIFIER NO
.../prog007_loja.cob:186:043  IDENTIFIER ADVANCING
.../prog007_loja.cob:186:052  '.'
.../prog007_loja.cob:187:012  KEYWORD ACCEPT
.../prog007_loja.cob:187:019  IDENTIFIER WS-QTD
.../prog007_loja.cob:187:025  '.'
.../prog007_loja.cob:188:012  KEYWORD DISPLAY
.../prog007_loja.cob:188:020  STRING "DESCRICAO: "
.../prog007_loja.cob:188:034  IDENTIFIER WITH
.../prog007_loja.cob:188:039  IDENTIFIER NO
.../prog007_loja.cob:188:042  IDENTIFIER ADVANCING
.../prog007_loja.cob:188:051  '.'
.../prog007_loja.cob:189:012  KEYWORD ACCEPT
.../prog007_loja.cob:189:019  IDENTIFIER WS-DESCRICAO
.../prog007_loja.cob:189:031  '.'
.../prog007_loja.cob:191:012  IDENTIFIER ADD
.../prog007_loja.cob:191:016  NUMBER 1
.../prog007_loja.cob:191:018  IDENTIFIER TO
.../prog007_loja.cob:191:021  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:191:034  '.'
.../prog007_loja.cob:192:012  KEYWORD MOVE
.../prog007_loja.cob:192:017  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:192:031  IDENTIFIER TO
.../prog007_loja.cob:192:034  IDENTIFIER WS-PID
.../prog007_loja.cob:192:040  '.'
.../prog007_loja.cob:194:012  KEYWORD DISPLAY
.../prog007_loja.cob:194:020  STRING "--------------"
.../prog007_loja.cob:195:012  KEYWORD DISPLAY
.../prog007_loja.cob:195:020  IDENTIFIER WS-PRODUTO
.../prog007_loja.cob:195:030  '.'
.../prog007_loja.cob:196:012  KEYWORD DISPLAY
.../prog007_loja.cob:196:020  STRING "--------------"
.../prog007_loja.cob:198:012  IDENTIFIER WRITE
.../prog007_loja.cob:198:018  IDENTIFIER REG-PRODUTO
.../prog007_loja.cob:198:030  IDENTIFIER FROM
.../prog007_loja.cob:198:035  IDENTIFIER WS-PRODUTO
.../prog007_loja.cob:199:015  IDENTIFIER INVALID
.../prog007_loja.cob:199:023  IDENTIFIER KEY
.../prog007_loja.cob:200:018  KEYWORD DISPLAY
.../prog007_loja.cob:200:026  STRING "CODIGO DE PRODUTO JAH EXISTE"
.../prog007_loja.cob:200:057  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:201:018  KEYWORD MOVE
.../prog007_loja.cob:201:023  NUMBER 1
.../prog007_loja.cob:201:025  IDENTIFIER TO
.../prog007_loja.cob:201:028  IDENTIFIER WS-EXIT
.../prog007_loja.cob:202:015  IDENTIFIER NOT
.../prog007_loja.cob:202:019  IDENTIFIER INVALID
.../prog007_loja.cob:202:027  IDENTIFIER KEY
.../prog007_loja.cob:203:018  KEYWORD DISPLAY
.../prog007_loja.cob:203:026  STRING "PRODUTO ARMAZENADO"
.../prog007_loja.cob:203:046  '.'
.../prog007_loja.cob:205:008  IDENTIFIER 00002-SELECT-PRODUTO
.../prog007_loja.cob:205:028  '.'
.../prog007_loja.cob:206:012  KEYWORD DISPLAY
.../prog007_loja.cob:206:020  STRING "CONSULTA PRODUTO"
.../prog007_loja.cob:206:038  '.'
.../prog007_loja.cob:207:012  KEYWORD DISPLAY
.../prog007_loja.cob:207:020  STRING "WS-ULTIMO-PID --> "
.../prog007_loja.cob:207:041  IDENTIFIER WS-ULTIMO-PID
.../prog007_loja.cob:207:054  '.'
.../prog007_loja.cob:208:012  KEYWORD DISPLAY
.../prog007_loja.cob:208:020  STRING "PID: "
.../prog007_loja.cob:208:027  '.'
.../prog007_loja.cob:209:012  KEYWORD ACCEPT
.../prog007_loja.cob:209:019  IDENTIFIER WS-PID
.../prog007_loja.cob:209:025  '.'
.../prog007_loja.cob:211:012  KEYWORD MOVE
.../prog007_loja.cob:211:017  IDENTIFIER WS-PID
.../prog007_loja.cob:211:024  IDENTIFIER TO
.../prog007_loja.cob:211:027  IDENTIFIER RP-PID
.../prog007_loja.cob:211:033  '.'
.../prog007_loja.cob:213:012  IDENTIFIER READ
.../prog007_loja.cob:213:017  IDENTIFIER FD-PRODUTOS
.../prog007_loja.cob:213:029  IDENTIFIER RECORD
.../prog007_loja.cob:213:036  IDENTIFIER KEY
.../prog007_loja.cob:213:040  IDENTIFIER RP-PID
.../prog007_loja.cob:214:017  IDENTIFIER INVALID
.../prog007_loja.cob:214:025  IDENTIFIER KEY
.../prog007_loja.cob:215:021  KEYWORD DISPLAY
.../prog007_loja.cob:215:029  STRING "ERRO AO LOCALIZAR REGISTRO"
.../prog007_loja.cob:216:017  IDENTIFIER NOT
.../prog007_loja.cob:216:021  IDENTIFIER INVALID
.../prog007_loja.cob:216:029  IDENTIFIER KEY
.../prog007_loja.cob:217:021  KEYWORD MOVE
.../prog007_loja.cob:217:026  IDENTIFIER REG-PRODUTO
.../prog007_loja.cob:217:038  IDENTIFIER TO
.../prog007_loja.cob:217:041  IDENTIFIER WS-PRODUTO
.../prog007_loja.cob:218:021  KEYWORD DISPLAY
.../prog007_loja.cob:218:029  STRING " "
.../prog007_loja.cob:219:021  KEYWORD DISPLAY
.../prog007_loja.cob:219:029  STRING "NOME: "
.../prog007_loja.cob:219:038  IDENTIFIER WITH
.../prog007_loja.cob:219:043  IDENTIFIER NO
.../prog007_loja.cob:219:046  IDENTIFIER ADVANCING
.../prog007_loja.cob:220:021  KEYWORD DISPLAY
.../prog007_loja.cob:220:029  IDENTIFIER WS-NOME
.../prog007_loja.cob:221:021  KEYWORD DISPLAY
.../prog007_loja.cob:221:029  STRING "VALOR: "
.../prog007_loja.cob:221:039  IDENTIFIER WITH
.../prog007_loja.cob:221:044  IDENTIFIER NO
.../prog007_loja.cob:221:047  IDENTIFIER ADVANCING
.../prog007_loja.cob:222:021  KEYWORD DISPLAY
.../prog007_loja.cob:222:029  IDENTIFIER WS-VALOR
.../prog007_loja.cob:223:021  KEYWORD DISPLAY
.../prog007_loja.cob:223:029  STRING "QUANTIDADE: "
.../prog007_loja.cob:223:044  IDENTIFIER WITH
.../prog007_loja.cob:223:049  IDENTIFIER NO
.../prog007_loja.cob:223:052  IDENTIFIER ADVANCING
.../prog007_loja.cob:224:021  KEYWORD DISPLAY
.../prog007_loja.cob:224:029  IDENTIFIER WS-QTD
.../prog007_loja.cob:225:021  KEYWORD DISPLAY
.../prog007_loja.cob:225:029  STRING "DESCRICAO: "
.../prog007_loja.cob:225:043  IDENTIFIER WITH
.../prog007_loja.cob:225:048  IDENTIFIER NO
.../prog007_loja.cob:225:051  IDENTIFIER ADVANCING
.../prog007_loja.cob:226:021  KEYWORD DISPLAY
.../prog007_loja.cob:226:029  IDENTIFIER WS-DESCRICAO
.../prog007_loja.cob:226:041  '.'
.../prog007_loja.cob:228:008  IDENTIFIER END
.../prog007_loja.cob:228:012  IDENTIFIER PROGRAM
.../prog007_loja.cob:228:020  IDENTIFIER PROG007_LOJA
.../prog007_loja.cob:228:032  '.'
```

= Resultados de erros

== Procedimento reproduzível

Para demonstrar a recuperação e a emissão de diagnósticos, foi criado o arquivo `documents/lexer-errors.cob` com o conteúdo abaixo. Cada linha introduz um caso inválido independente. A coluna 7 é o indicador de formato fixo; na primeira linha ela contém intencionalmente `X`, enquanto nas demais contém um espaço.

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

No total, há dez casos de erro diferentes.

== Casos incluídos

#figure(
  table(
    columns: 3,
    [*Linha*], [*Erro inserido*], [*Diagnóstico esperado*],
    [`000100`], [Indicador inválido na coluna 7], [`source::invalid_indicator`],
    [`000200`], [Caractere `@` não reconhecido], [`lexer::illegal_character`],
    [`000300`], [Caractere `#` não reconhecido], [`lexer::illegal_character`],
    [`000400`], [Identificador terminado em hífen], [`lexer::trailing_hyphen`],
    [`000500`], [Identificador com 31 caracteres], [`lexer::identifier_too_long`],
    [`000600`], [Literal string sem aspas de fechamento], [`lexer::unterminated_string`],
    [`000700`], [Cláusula `PIC` sem picture], [`lexer::missing_picture`],
    [`000800`], [Caractere `_` no início de uma palavra], [`lexer::illegal_character`],
    [`000900`], [Copybook inexistente], [`preprocess::copybook_error`],
    [`001000`], [Cláusula `COPY ... REPLACING` \ não suportada], [`preprocess::replacing_unsupported`],
  ),
  caption: [Casos de erro para execução manual; devem ser confirmados pela saída real do programa.],
) <error-cases>

Os dez casos são esperados como diagnósticos de erro. As ocorrências de `@`, `#` e `_` são três instâncias do mesmo código de caractere ilegal. O último caso verifica uma diretiva ainda não suportada e não uma falha de leitura ou de tokenização.

== Saída dos erros

Executando:

```sh
cargo run -- lex documents/lexer-errors.cob
```

O comando terminou com _exit code_ igual a 1. A sua saída foi:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running `target/debug/cobol-compiler lex documents/lexer-errors.cob`
cobol::source::invalid_indicator

  × invalid indicator 'X' in column 7
   ╭─[documents/lexer-errors.cob:1:7]
 1 │ 000100X   DISPLAY 'invalid indicator'.
   ·       ┬
   ·       ╰── expected ' ', '*', '/', '-' or 'D'
 2 │ 000200  DISPLAY @.
   ╰────

cobol::preprocess::copybook_error

  × copybook 'NO_SUCH_BOOK' not found
    ╭─[documents/lexer-errors.cob:9:9]
  8 │ 000800  _BAD.
  9 │ 000900  COPY NO_SUCH_BOOK.
    ·         ─────────┬────────
    ·                  ╰── while expanding this COPY
 10 │ 001000  COPY BOOK REPLACING ==A== BY ==B==.
    ╰────
  help: add a search directory with -I <dir>

cobol::preprocess::replacing_unsupported

  × COPY ... REPLACING is not supported
    ╭─[documents/lexer-errors.cob:10:9]
  9 │ 000900  COPY NO_SUCH_BOOK.
 10 │ 001000  COPY BOOK REPLACING ==A== BY ==B==.
    ·         ─────────────────┬─────────────────
    ·                          ╰── statement skipped
    ╰────

cobol::lexer::illegal_character

  × illegal character '@'
   ╭─[documents/lexer-errors.cob:2:17]
 1 │ 000100X   DISPLAY 'invalid indicator'.
 2 │ 000200  DISPLAY @.
   ·                 ┬
   ·                 ╰── not part of the COBOL character set
 3 │ 000300  DISPLAY #.
   ╰────

cobol::lexer::illegal_character

  × illegal character '#'
   ╭─[documents/lexer-errors.cob:3:17]
 2 │ 000200  DISPLAY @.
 3 │ 000300  DISPLAY #.
   ·                 ┬
   ·                 ╰── not part of the COBOL character set
 4 │ 000400  MOVE BAD- TO X.
   ╰────

cobol::lexer::trailing_hyphen

  × word 'BAD-' ends with a hyphen
   ╭─[documents/lexer-errors.cob:4:14]
 3 │ 000300  DISPLAY #.
 4 │ 000400  MOVE BAD- TO X.
   ·              ──┬─
   ·                ╰── words cannot end with '-'
 5 │ 000500  ABCDEFGHIJKLMNOPQRSTUVWXYZ12345.
   ╰────

cobol::lexer::identifier_too_long

  × word is 31 characters long (maximum is 30)
   ╭─[documents/lexer-errors.cob:5:9]
 4 │ 000400  MOVE BAD- TO X.
 5 │ 000500  ABCDEFGHIJKLMNOPQRSTUVWXYZ12345.
   ·         ───────────────┬───────────────
   ·                        ╰── too long
 6 │ 000600  DISPLAY 'unterminated
   ╰────

cobol::lexer::unterminated_string

  × unterminated string literal
   ╭─[documents/lexer-errors.cob:6:17]
 5 │ 000500  ABCDEFGHIJKLMNOPQRSTUVWXYZ12345.
 6 │ 000600  DISPLAY 'unterminated
   ·                 ──────┬──────
   ·                       ╰── literal is missing its closing quote
 7 │ 000700  PIC .
   ╰────

cobol::lexer::missing_picture

  × expected a picture string after PIC
   ╭─[documents/lexer-errors.cob:7:9]
 6 │ 000600  DISPLAY 'unterminated
 7 │ 000700  PIC .
   ·         ─┬─
   ·          ╰── picture string missing
 8 │ 000800  _BAD.
   ╰────

cobol::lexer::illegal_character

  × illegal character '_'
   ╭─[documents/lexer-errors.cob:8:9]
 7 │ 000700  PIC .
 8 │ 000800  _BAD.
   ·         ┬
   ·         ╰── not part of the COBOL character set
 9 │ 000900  COPY NO_SUCH_BOOK.
   ╰────

documents/lexer-errors.cob:002:009  KEYWORD DISPLAY
documents/lexer-errors.cob:002:018  '.'
documents/lexer-errors.cob:003:009  KEYWORD DISPLAY
documents/lexer-errors.cob:003:018  '.'
documents/lexer-errors.cob:004:009  KEYWORD MOVE
documents/lexer-errors.cob:004:019  IDENTIFIER TO
documents/lexer-errors.cob:004:022  IDENTIFIER X
documents/lexer-errors.cob:004:023  '.'
documents/lexer-errors.cob:005:040  '.'
documents/lexer-errors.cob:006:009  KEYWORD DISPLAY
documents/lexer-errors.cob:007:009  PICTURE
documents/lexer-errors.cob:007:013  '.'
documents/lexer-errors.cob:008:010  IDENTIFIER BAD
documents/lexer-errors.cob:008:013  '.'
```

= Conclusão

== Análise dos testes

Os 101 testes unitários, o teste de integração com _goldens_ sobre os 7 programas de exemplo e os 10 casos de erro mostram que o analisador léxico (com o leitor de formato fixo e o pré-processador `COPY`) produz a sequência esperada de _tokens_, informa a posição (arquivo, linha e coluna) de cada um e se recupera de erros, continuando a análise e coletando todos os diagnósticos em vez de parar no primeiro.

=== Palavras-chave registradas como identificadores

A seção 2.2 da especificação reserva apenas 14 palavras: `IDENTIFICATION`, `ENVIRONMENT`, `DATA`, `PROCEDURE`, `DIVISION`, `SECTION`, `DISPLAY`, `ACCEPT`, `PERFORM`, `STOP`, `RUN`, `IF`, `ELSE` e `MOVE`. Os programas de exemplo, porém, usam muitas outras palavras que o COBOL trata como reservadas, mas que não constam na especificação. Como o léxico segue estritamente a especificação, elas são emitidas como `IDENTIFIER`. Exemplos vistos nos _goldens_:

- `PROGRAM-ID`, `WORKING-STORAGE`, `FILE-CONTROL`;
- `END-IF`, `END`, `PROGRAM`;
- `TO`, `AND`, `VALUE`, `OF`;
- verbos como `COMPUTE`, `ADD`, `OPEN` e `READ`.

No futuro, uma lista mais completa de palavras reservadas poderia ser mantida para que o analisador sintático pudesse ter um contexto melhor, e reportar corretamente erros onde identificadores colidem com palavras reservadas.

=== Desvios da especificação: _underscores_ em identificadores

A especificação define identificadores apenas com letras, dígitos e hífens. Os exemplos, no entanto, contêm nomes como `PROG008_STRING`, `PROG006_FILES` e `PROG007_LOJA` (em `PROGRAM-ID` e `END PROGRAM`, nos programas 5 a 7). Rejeitá-los com erro impediria a análise de programas fornecidos para teste. A decisão foi aceitar o sublinhado no meio do identificador, emitindo um _warning_ (`cobol::lexer::underscore_in_identifier`) em vez de um erro. O identificador é gerado normalmente, o código de saída continua 0 e o aviso aparece na seção `DIAGNOSTICS:` dos _goldens_. Um sublinhado no início do identificador continua sendo erro (`illegal_character`), e o sublinhado conta para o limite de 30 caracteres.

De forma semelhante, as entradas de comentário da `IDENTIFICATION DIVISION` (`AUTHOR`, `REMARKS` etc.) são emitidas como um único _token_ `CommentEntry`, pois seu texto livre não segue a gramática da especificação.

=== Evolução do método de teste

Os _goldens_ atuais são simples: um arquivo de texto por programa, comparado por igualdade, indicando quais arquivos diferem. Isso é suficiente para a saída do léxico, que é linear e curta. Nas fases seguintes (árvore sintática, tabelas semânticas, código gerado), as saídas serão bem maiores e precisarão ser revisadas com cuidado a cada mudança. Nesse cenário vale substituir o mecanismo atual por uma _crate_ especializada como a _insta_, que oferece _snapshots_ com diferenças detalhadas, revisão interativa (`cargo insta review`), aceitação seletiva de mudanças e redação de partes instáveis da saída. A migração é simples, pois o formato textual atual poderia ser reaproveitado como _snapshot_.

== Limitações e próximas etapas

- *Léxico:* não distingue Área A e Área B, não suporta formato livre, `COPY ... REPLACING`, `REPLACE` nem diretivas de compilador.
- *Parser (fase 2):* construir a árvore sintática a partir dos _tokens_, interpretando as palavras sensíveis ao contexto citadas acima, níveis, nomes de parágrafos e sinais de literais numéricos.
- *Análise semântica (fase 3):* verificar compatibilidade em `MOVE`, unicidade de identificadores, alvos de `PERFORM`/`GO TO` e associação de arquivos.
- *Geração de código (fase 4):* traduzir para C ou gerar _bytecode_/código de máquina; a escolha do _backend_ ainda não foi feita.
