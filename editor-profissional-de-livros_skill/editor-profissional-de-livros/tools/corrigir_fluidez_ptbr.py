#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
CORRETOR DE FLUIDEZ PT-BR — TRADUÇÕES LITERÁRIAS
Parte da skill: editor-profissional-de-livros

COMO USAR:
1. Copie este arquivo para a raiz do seu projeto de tradução
2. Ajuste a variável PASTA_TRADUCAO se necessário
3. Execute: python corrigir_fluidez_ptbr.py

ESTRUTURA ESPERADA:
projeto/
  ├── traducao_pt_br/epub_build/text/  (ou ajuste PASTA_TRADUCAO)
  │   ├── part0001.html
  │   ├── part0002.html
  │   └── ...
  └── corrigir_fluidez_ptbr.py (este arquivo)
"""

import os
import re
from pathlib import Path

# ============================================================================
# CONFIGURAÇÃO — AJUSTE CONFORME SEU PROJETO
# ============================================================================

PASTA_TRADUCAO = 'traducao_pt_br/epub_build/text'  # Pasta com arquivos HTML traduzidos

# ============================================================================
# CORREÇÕES — NÃO ALTERAR (baseado em análise de 15+ bestsellers)
# ============================================================================

# ANGLICISMOS DE MOVIMENTO
ANGLICISMOS_MOVIMENTO = [
    (r'\bAlcançando\b', 'Estendendo a mão'),
    (r'\balcançando\b', 'estendendo a mão'),
    (r'\balcançou e\b', 'estendeu a mão e'),
    (r'\balcançou para\b', 'estendeu a mão para'),
    (r'\bEnrolando os braços no\b', 'Passando os braços ao redor do'),
    (r'\bEnrolando os braços na\b', 'Passando os braços ao redor da'),
    (r'\benrolando os braços no\b', 'passando os braços ao redor do'),
    (r'\benrolando os braços na\b', 'passando os braços ao redor da'),
    (r'\bEnrolando os braços mais apertados no\b', 'Apertando os braços ao redor do'),
    (r'\bse enrolou no quadril\b', 'agarrou o quadril'),
    (r'\bse enrolou na cintura\b', 'agarrou a cintura'),
    (r'\bse enrolou ao redor dele\b', 'se agarrou a ele'),
    (r'\bse enrolou ao redor dela\b', 'se agarrou a ela'),
    (r'\benganou as coxas sobre\b', 'colocou as coxas sobre'),
    (r'\benganar as coxas sobre\b', 'colocar as coxas sobre'),
    (r'\bEnganando uma das pernas\b', 'Colocando uma das pernas'),
    (r'\benganando uma das pernas\b', 'colocando uma das pernas'),
    (r'\bas enganou sobre\b', 'as colocou sobre'),
    (r'\ba cavaleiro no colo\b', 'montada no colo'),
    (r'\ba cavaleiro sobre\b', 'montada sobre'),
    (r'\bsentando a cavaleiro\b', 'sentando montada'),
    (r'\bse sentou a cavaleiro\b', 'se sentou montada'),
]

# ANGLICISMOS DE EXPRESSÃO
ANGLICISMOS_EXPRESSAO = [
    (r'\barrastou as palavras\b', 'disse arrastado'),
    (r'\barrastar as palavras\b', 'falar arrastado'),
    (r'\bnuzzlar\b', 'roçar o rosto'),
    (r'\bnuzzlou\b', 'roçou o rosto'),
    (r'\bnuzzlando\b', 'roçando o rosto'),
    (r'\bos lábios enrolados sob os dentes\b', 'mordia os lábios'),
    (r'\blábios enrolados sob os dentes\b', 'mordendo os lábios'),
    (r'\bSuavizando a voz\b', 'Com a voz mais suave'),
    (r'\bsuavizando a voz\b', 'com a voz mais suave'),
    (r'\bclareou a garganta\b', 'pigarreou'),
    (r'\bClarear a garganta\b', 'Pigarrear'),
    (r'\barqueou uma sobrancelha desafiadoramente\b', 'ergueu uma sobrancelha desafiadora'),
    (r'\barqueou uma sobrancelha\b', 'ergueu uma sobrancelha'),
]

# ANGLICISMOS DE PERCEPÇÃO
ANGLICISMOS_PERCEPCAO = [
    (r'\bInclinando a cabeça para trás\b', 'Erguendo a cabeça'),
    (r'\binclinando a cabeça para trás\b', 'erguendo a cabeça'),
    (r'\bForçando as pálpebras a abrirem\b', 'Abrindo os olhos com esforço'),
    (r'\bforçando as pálpebras a abrirem\b', 'abrindo os olhos com esforço'),
    (r'\bForçando os olhos a abrirem\b', 'Abrindo os olhos com esforço'),
    (r'\bforçando os olhos a abrirem\b', 'abrindo os olhos com esforço'),
]

# CONSTRUÇÕES ERÓTICAS LITERAIS
CONSTRUCOES_EROTICAS = [
    (r'\bse sobrepondo ao\b', 'somando-se ao'),
    (r'\bse sobrepondo à\b', 'somando-se à'),
    (r'\bbombeando-a cheia do clímax\b', 'enchendo-a com o gozo'),
    (r'\bbombeando-a cheia de\b', 'enchendo-a com'),
    (r'\bempurrões curtos e martelantes\b', 'estocadas curtas e brutais'),
    (r'\bem empurrões curtos\b', 'em estocadas curtas'),
    (r'\bno compasso dos empurrões\b', 'no compasso das estocadas'),
    (r'\bondulou os quadris sobre ele\b', 'ondulou os quadris dela sobre ele'),
    (r'\bse enrolava cada vez mais apertado\b', 'crescia cada vez mais intenso'),
    (r'\bum orgasmo se enrolou\b', 'um orgasmo cresceu'),
    (r'\ba tensão se enrolando\b', 'a tensão crescendo'),
]

# OUTRAS MELHORIAS
OUTRAS_MELHORIAS = [
    (r'\bCambaleando nos pés\b', 'Cambaleando'),
    (r'\bcambaleando nos pés\b', 'cambaleando'),
    (r'\btropeçou ao lado de\b', 'seguiu ao lado de'),
    (r'\bsoltou um bafo explosivo\b', 'soltou o ar num suspiro explosivo'),
    (r'\bsoltou um bafo de alívio\b', 'suspirou aliviada'),
    (r'\bsoltou um bafo de\b', 'soltou o ar com'),
    (r'\bsoltou um bafo\b', 'soltou o ar'),
    (r'\bSoltando o ar\b', 'Respirando fundo'),
    (r'\bsoltando o ar\b', 'respirando fundo'),
    (r'\bGalvanizados\b', 'Animados'),
    (r'\bgalvanizados\b', 'animados'),
    (r'\bbalançando-a nos braços\b', 'pegando-a nos braços'),
    (r'\bbalançando-o nos braços\b', 'pegando-o nos braços'),
    (r'\ba erguendo de e para longe\b', 'erguendo-a e afastando'),
    (r'\ba erguendo e afastando\b', 'erguendo-a e afastando'),
]

# REDUNDÂNCIAS
REDUNDANCIAS = [
    (r'\bse agarravam da buceta\b', 'se agarravam'),
    (r'\bse agarravam do pau\b', 'se agarravam'),
]

# Combinar todas as correções
TODAS_CORRECOES = (
    ANGLICISMOS_MOVIMENTO +
    ANGLICISMOS_EXPRESSAO +
    ANGLICISMOS_PERCEPCAO +
    CONSTRUCOES_EROTICAS +
    OUTRAS_MELHORIAS +
    REDUNDANCIAS
)

# ============================================================================
# FUNÇÕES
# ============================================================================

def corrigir_arquivo(filepath):
    """Aplica todas as correções em um arquivo"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            conteudo = f.read()
    except Exception as e:
        print(f"❌ Erro ao ler {filepath.name}: {e}")
        return 0
    
    conteudo_original = conteudo
    alteracoes = 0
    
    for padrao, substituicao in TODAS_CORRECOES:
        conteudo_novo = re.sub(padrao, substituicao, conteudo)
        if conteudo_novo != conteudo:
            alteracoes += len(re.findall(padrao, conteudo))
            conteudo = conteudo_novo
    
    if conteudo != conteudo_original:
        try:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(conteudo)
            return alteracoes
        except Exception as e:
            print(f"❌ Erro ao escrever {filepath.name}: {e}")
            return 0
    
    return 0

def main():
    """Processa todos os arquivos HTML na pasta de tradução"""
    pasta = Path(PASTA_TRADUCAO)
    
    if not pasta.exists():
        print(f"❌ Pasta não encontrada: {pasta}")
        print(f"\n💡 Ajuste a variável PASTA_TRADUCAO no início do script")
        print(f"   Pasta atual configurada: {PASTA_TRADUCAO}")
        return
    
    arquivos = sorted(pasta.glob('*.html'))
    
    if not arquivos:
        print(f"❌ Nenhum arquivo .html encontrado em: {pasta}")
        return
    
    print("🔍 CORRETOR DE FLUIDEZ PT-BR")
    print("=" * 60)
    print(f"📁 Pasta: {pasta}")
    print(f"📄 Arquivos encontrados: {len(arquivos)}")
    print(f"🔧 Regras de correção: {len(TODAS_CORRECOES)}")
    print("=" * 60)
    print()
    
    total_alteracoes = 0
    arquivos_alterados = 0
    
    for arquivo in arquivos:
        alteracoes = corrigir_arquivo(arquivo)
        if alteracoes > 0:
            arquivos_alterados += 1
            total_alteracoes += alteracoes
            print(f"✓ {arquivo.name}: {alteracoes} correções")
    
    print()
    print("=" * 60)
    print(f"✅ CONCLUÍDO!")
    print(f"📊 {arquivos_alterados} arquivos alterados de {len(arquivos)} total")
    print(f"📊 {total_alteracoes} correções aplicadas")
    print("=" * 60)
    
    if arquivos_alterados > 0:
        print()
        print("📝 PRÓXIMOS PASSOS:")
        print("   1. Revise as alterações nos arquivos")
        print("   2. Regenere o EPUB se necessário")
        print("   3. Teste no Kindle Previewer ou Calibre")

if __name__ == '__main__':
    main()
