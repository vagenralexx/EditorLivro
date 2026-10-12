import { Component, type ErrorInfo, type ReactNode } from "react";

interface State { error: Error | null; }

/** Evita a tela em branco: se um componente falhar ao desenhar, o autor vê o que fazer. Nada em disco é tocado. */
export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State { return { error }; }

  componentDidCatch(error: Error, info: ErrorInfo) { console.error("Falha ao desenhar a interface:", error, info.componentStack); }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <main className="fatal-error" role="alert">
        <h1>Algo saiu do esperado</h1>
        <p>A tela falhou ao ser desenhada, mas <b>nenhum arquivo foi alterado</b>. Edições feitas desde o último salvamento nesta janela podem não ter sido gravadas; os backups ficam em <code>.livro/backups</code> na pasta do projeto.</p>
        <pre>{this.state.error.message}</pre>
        <button className="primary-button" type="button" onClick={() => window.location.reload()}>Recarregar o app</button>
      </main>
    );
  }
}
