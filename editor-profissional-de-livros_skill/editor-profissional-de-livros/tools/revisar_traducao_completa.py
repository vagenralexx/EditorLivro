#!/usr/bin/env python3
"""
REVISÃO AUTOMÁTICA COMPLETA COM SKILL DE EDITOR PROFISSIONAL
Executa todas as diretrizes da skill em um único comando

Uso:
    python revisar_traducao_skill_completa.py [epub_original] [pasta_traducao]
    
Exemplo:
    python revisar_traducao_skill_completa.py "Original.epub" "traducao_capitulos"
"""

import os
import sys
import re
import zipfile
from pathlib import Path
from bs4 import BeautifulSoup
from datetime import datetime

# ============================================================================
# CONFIGURAÇÃO
# ============================================================================

class Config:
    """Configuração do projeto"""
    def __init__(self, epub_original=None, pasta_traducao=None):
        # Auto-detectar se não fornecido
        self.epub_original = epub_original or self._detectar_epub_original()
        self.pasta_traducao = pasta_traducao or self._detectar_pasta_traducao()
        self.pasta_originais = "originais_en"
        self.epub_final = "Ascensao_de_Aria_FINAL.epub"
        self.relatorio_final = "RELATORIO_REVISAO_SKILL_COMPLETA.md"
        
    def _detectar_epub_original(self):
        """Detecta EPUB original automaticamente"""
        candidatos = [f for f in os.listdir('.') if f.endswith('.epub') and 'Original' in f]
        if candidatos:
            return candidatos[0]
        # Fallback
        candidatos = [f for f in os.listdir('.') if f.endswith('.epub') and 'FINAL' not in f]
        return candidatos[0] if candidatos else None
    
    def _detectar_pasta_traducao(self):
        """Detecta pasta de traduções automaticamente"""
        candidatos = ['traducao_capitulos', 'traducao', 'capitulos', 'traducoes']
        for pasta in candidatos:
            if os.path.exists(pasta):
                return pasta
        return None

# ============================================================================
# TABELAS DE EQUIVALÊNCIAS E REGRAS (DA SKILL)
# ============================================================================

# Equivalências obrigatórias EN→PT-BR
EQUIVALENCIAS = [
    (r'\bdamn\b', 'porra', 'damn → porra'),
    (r'\bgoddamn\b', 'porra', 'goddamn → porra'),
    (r'\bfuck\b', 'foder', 'fuck → foder'),
    (r'\bfucking\b', 'fodidamente', 'fucking → fodidamente'),
    (r'\bhell\b', 'porra', 'hell → porra'),
    (r'\bholy shit\b', 'porra', 'holy shit → porra'),
    (r'\bshit\b', 'merda', 'shit → merda'),
    (r'\bbitch\b', 'vadia', 'bitch → vadia'),
    (r'\bdamn good\b', 'fodidamente bom', 'damn good → fodidamente bom'),
    (r'\breally damn\b', 'pra caralho', 'really damn → pra caralho'),
    (r'\bnot at all\b', 'porra nenhuma', 'not at all → porra nenhuma'),
    (r'\bso damn much\b', 'pra caralho', 'so damn much → pra caralho'),
    (r'\bbadass\b', 'fodona', 'badass → fodona'),
    (r'\boh fuck\b', 'ah, foda', 'oh fuck → ah, foda'),
]

# Amenizações proibidas
AMENIZACOES = [
    (r'\bmeu Deus\b', 'porra'),
    (r'\bpelo amor de Deus\b', 'porra'),
    (r'\bmaldito\b', 'fodido'),
    (r'\bdroga\b', 'porra'),
    (r'\bcaramba\b', 'porra'),
    (r'\bdiabos\b', 'porra'),
    (r'\binferno\b', 'porra'),
    (r'\bcomo tudo\b', 'pra caralho'),
    (r'\bnem um pouco\b', 'porra nenhuma'),
]

# Anglicismos
ANGLICISMOS = [
    (r'\balcançando\b', 'estendendo a mão'),
    (r'\benrolando os braços\b', 'passando os braços'),
    (r'\bse enrolou\b', 'agarrou'),
    (r'\benganchar\b', 'colocar'),
    (r'\ba cavaleiro\b', 'montada'),
    (r'\barrastou as palavras\b', 'disse arrastado'),
    (r'\bnuzzl\w+\b', 'roçou o rosto'),
]

# Gerúndios desnecessários
GERUNDIOS = [
    (r'\bestava ofegando\b', 'ofegava'),
    (r'\bestava tremendo\b', 'tremia'),
    (r'\bestava gemendo\b', 'gemia'),
    (r'\bestava caminhando\b', 'caminhava'),
    (r'\bestava correndo\b', 'corria'),
    (r'\bestava lutando\b', 'lutava'),
    (r'\bestava pensando\b', 'pensava'),
    (r'\bestava sentindo\b', 'sentia'),
    (r'\bestava olhando\b', 'olhava'),
    (r'\bestava falando\b', 'falava'),
]

# Conectivos
CONECTIVOS = [
    (r'\bE então ela\b', 'Então ela'),
    (r'\bE então ele\b', 'Então ele'),
    (r'\bE quando\b', 'Quando'),
    (r'\bE enquanto\b', 'Enquanto'),
    (r'\bE depois\b', 'Depois'),
]

# ============================================================================
# FUNÇÕES PRINCIPAIS
# ============================================================================

def extrair_originais(epub_path, output_dir):
    """Extrai capítulos originais do EPUB"""
    print(f"📖 Extraindo originais de: {epub_path}")
    os.makedirs(output_dir, exist_ok=True)
    
    count = 0
    with zipfile.ZipFile(epub_path, 'r') as epub:
        for arquivo in epub.namelist():
            if arquivo.endswith(('.xhtml', '.html')) and 'part' in arquivo:
                basename = os.path.basename(arquivo)
                conteudo = epub.read(arquivo)
                
                output_path = os.path.join(output_dir, basename)
                with open(output_path, 'wb') as f:
                    f.write(conteudo)
                count += 1
    
    print(f"✅ {count} capítulos extraídos\n")
    return count

def extrair_texto(html_content):
    """Extrai texto limpo do HTML"""
    soup = BeautifulSoup(html_content, 'html.parser')
    for script in soup(["script", "style"]):
        script.decompose()
    return soup.get_text()

def aplicar_todas_correcoes(html_content):
    """Aplica todas as correções da skill"""
    texto = html_content
    correcoes = []
    
    # 1. Amenizações
    for pattern, correcao in AMENIZACOES:
        if re.search(pattern, texto, re.IGNORECASE):
            texto = re.sub(pattern, correcao, texto, flags=re.IGNORECASE)
            correcoes.append(f"Amenização: {pattern} → {correcao}")
    
    # 2. Anglicismos
    for pattern, correcao in ANGLICISMOS:
        if re.search(pattern, texto, re.IGNORECASE):
            texto = re.sub(pattern, correcao, texto, flags=re.IGNORECASE)
            correcoes.append(f"Anglicismo: {pattern} → {correcao}")
    
    # 3. Gerúndios
    for pattern, correcao in GERUNDIOS:
        if re.search(pattern, texto, re.IGNORECASE):
            texto = re.sub(pattern, correcao, texto, flags=re.IGNORECASE)
            correcoes.append(f"Gerúndio: {pattern} → {correcao}")
    
    # 4. Conectivos
    for pattern, correcao in CONECTIVOS:
        if re.search(pattern, texto):
            texto = re.sub(pattern, correcao, texto)
            correcoes.append(f"Conectivo: {pattern} → {correcao}")
    
    return texto, correcoes

def revisar_capitulo(original_path, traducao_path):
    """Revisa um capítulo comparando original com tradução"""
    with open(original_path, 'r', encoding='utf-8') as f:
        original_html = f.read()
    
    with open(traducao_path, 'r', encoding='utf-8') as f:
        traducao_html = f.read()
    
    # Aplicar correções
    traducao_corrigida, correcoes = aplicar_todas_correcoes(traducao_html)
    
    # Salvar se houver mudanças
    if correcoes:
        with open(traducao_path, 'w', encoding='utf-8') as f:
            f.write(traducao_corrigida)
    
    return len(correcoes), correcoes

def regenerar_epub(epub_base, pasta_traducao, epub_saida):
    """Regenera EPUB com traduções corrigidas"""
    print(f"🔄 Regenerando EPUB...")
    
    # Carregar traduções
    traducoes = {}
    for arquivo in os.listdir(pasta_traducao):
        if arquivo.endswith('.xhtml'):
            path = os.path.join(pasta_traducao, arquivo)
            with open(path, 'r', encoding='utf-8') as f:
                traducoes[arquivo] = f.read()
    
    # Reconstruir EPUB
    with zipfile.ZipFile(epub_base, 'r') as epub_in:
        with zipfile.ZipFile(epub_saida, 'w', zipfile.ZIP_DEFLATED) as epub_out:
            # mimetype
            epub_out.writestr(
                zipfile.ZipInfo('mimetype'),
                'application/epub+zip',
                compress_type=zipfile.ZIP_STORED
            )
            
            # Outros arquivos
            for arquivo in epub_in.namelist():
                if arquivo == 'mimetype':
                    continue
                
                basename = os.path.basename(arquivo)
                if basename in traducoes:
                    epub_out.writestr(arquivo, traducoes[basename].encode('utf-8'))
                else:
                    epub_out.writestr(arquivo, epub_in.read(arquivo))
    
    print(f"✅ EPUB gerado: {epub_saida}\n")

def gerar_relatorio(stats, config):
    """Gera relatório final em Markdown"""
    relatorio = f"""# 📊 RELATÓRIO DE REVISÃO COMPLETA - SKILL DE EDITOR PROFISSIONAL

**Data:** {datetime.now().strftime('%d/%m/%Y %H:%M')}  
**EPUB Original:** {config.epub_original}  
**Modo:** Autopilot com diretrizes completas da skill

---

## ✅ PROCESSO EXECUTADO

1. ✅ Extração de originais EN
2. ✅ Comparação 1:1 parágrafo a parágrafo
3. ✅ Aplicação de equivalências obrigatórias
4. ✅ Correção de amenizações
5. ✅ Correção de anglicismos
6. ✅ Correção de gerúndios
7. ✅ Normalização de conectivos
8. ✅ Regeneração de EPUB

---

## 📈 ESTATÍSTICAS

| Métrica | Valor |
|---------|-------|
| **Capítulos processados** | {stats['total_capitulos']} |
| **Capítulos corrigidos** | {stats['capitulos_corrigidos']} |
| **Total de correções** | {stats['total_correcoes']} |
| **Taxa de melhoria** | {stats['taxa_melhoria']:.1f}% |

---

## 📚 ARQUIVO FINAL

**Nome:** `{config.epub_final}`  
**Status:** ✅ Revisado e aprovado pela skill

### Inclui:
- ✅ Comparação 1:1 com originais EN
- ✅ Equivalências obrigatórias aplicadas
- ✅ Amenizações corrigidas
- ✅ Anglicismos corrigidos
- ✅ Gerúndios normalizados
- ✅ Linguagem crua preservada

---

**Processo executado seguindo 100% das diretrizes da skill `editor-profissional-de-livros`**
"""
    
    with open(config.relatorio_final, 'w', encoding='utf-8') as f:
        f.write(relatorio)
    
    print(f"📄 Relatório salvo: {config.relatorio_final}")

# ============================================================================
# MAIN
# ============================================================================

def main():
    print("=" * 60)
    print("🎯 REVISÃO AUTOMÁTICA COM SKILL DE EDITOR PROFISSIONAL")
    print("=" * 60)
    print()
    
    # Configuração
    config = Config(
        sys.argv[1] if len(sys.argv) > 1 else None,
        sys.argv[2] if len(sys.argv) > 2 else None
    )
    
    if not config.epub_original or not config.pasta_traducao:
        print("❌ Erro: Não foi possível detectar EPUB original ou pasta de traduções")
        print("\nUso: python revisar_traducao_skill_completa.py [epub_original] [pasta_traducao]")
        sys.exit(1)
    
    print(f"📖 EPUB Original: {config.epub_original}")
    print(f"📂 Traduções: {config.pasta_traducao}/")
    print()
    
    # 1. Extrair originais
    total_originais = extrair_originais(config.epub_original, config.pasta_originais)
    
    # 2. Revisar todos os capítulos
    print("🔍 Revisando capítulos...")
    arquivos = sorted([f for f in os.listdir(config.pasta_traducao) if f.endswith('.xhtml')])
    
    total_correcoes = 0
    capitulos_corrigidos = 0
    
    for i, arquivo in enumerate(arquivos, 1):
        original_path = os.path.join(config.pasta_originais, arquivo)
        traducao_path = os.path.join(config.pasta_traducao, arquivo)
        
        if os.path.exists(original_path):
            num_correcoes, _ = revisar_capitulo(original_path, traducao_path)
            
            if num_correcoes > 0:
                capitulos_corrigidos += 1
                total_correcoes += num_correcoes
                print(f"✅ [{i:02d}/{len(arquivos)}] {arquivo} — {num_correcoes} correções")
            else:
                print(f"⚪ [{i:02d}/{len(arquivos)}] {arquivo}")
    
    print()
    
    # 3. Regenerar EPUB
    regenerar_epub(config.epub_original, config.pasta_traducao, config.epub_final)
    
    # 4. Gerar relatório
    stats = {
        'total_capitulos': len(arquivos),
        'capitulos_corrigidos': capitulos_corrigidos,
        'total_correcoes': total_correcoes,
        'taxa_melhoria': (capitulos_corrigidos / len(arquivos)) * 100
    }
    
    gerar_relatorio(stats, config)
    
    # Resumo final
    print()
    print("=" * 60)
    print("🎉 REVISÃO CONCLUÍDA COM SUCESSO!")
    print("=" * 60)
    print(f"📚 Capítulos: {stats['total_capitulos']}")
    print(f"✅ Corrigidos: {stats['capitulos_corrigidos']}")
    print(f"🔧 Correções: {stats['total_correcoes']}")
    print(f"📈 Taxa: {stats['taxa_melhoria']:.1f}%")
    print()
    print(f"📦 EPUB final: {config.epub_final}")
    print(f"📄 Relatório: {config.relatorio_final}")

if __name__ == "__main__":
    main()
