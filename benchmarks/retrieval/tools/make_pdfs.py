"""Regenerates the PDF documents of the retrieval benchmark corpus.

The PDFs are committed; run this only after editing their text below:
    python benchmarks/retrieval/tools/make_pdfs.py
Needs reportlab. `invariant=1` keeps the bytes reproducible (no dates or random ids).
"""

from pathlib import Path

from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import getSampleStyleSheet
from reportlab.platypus import Paragraph, SimpleDocTemplate, Spacer

CORPUS = Path(__file__).resolve().parent.parent / "corpus"

DOCS = {
    "pt/operacoes/manual-empilhadeira.pdf": (
        "Manual de Operação da Empilhadeira Elétrica",
        [
            "Documento fictício da Transportes Serra Azul.",
            "Capacidade: a carga máxima da empilhadeira elétrica do armazém é de 2.500 kg com o garfo padrão, "
            "com o centro de carga a 500 mm. Com o garfo estendido a carga máxima cai para 1.600 kg.",
            "Velocidade: dentro do armazém a velocidade máxima é de 10 km/h, e de 5 km/h nos corredores de pedestres.",
            "Check-list diário: antes do primeiro turno o operador confere bateria, buzina, freio, luzes e o cinto de segurança, "
            "e registra o check-list no tablet da doca.",
            "Habilitação: só operadores com certificado interno podem dirigir a empilhadeira. "
            "O certificado deve ser renovado a cada 2 anos.",
        ],
    ),
    "pt/clientes/tabela-frete-2026.pdf": (
        "Tabela de Frete Fracionado 2026",
        [
            "Documento fictício da Transportes Serra Azul. Valores válidos de 01/01/2026 a 31/12/2026.",
            "Região Sul: R$ 0,85 por kg. Região Sudeste: R$ 0,70 por kg. Região Nordeste: R$ 1,40 por kg. "
            "Região Norte: R$ 1,95 por kg.",
            "Frete mínimo: R$ 120,00 por coleta, qualquer que seja o peso.",
            "Taxa de agendamento de entrega: R$ 35,00 por nota fiscal. "
            "Seguro (ad valorem): 0,3% do valor da nota fiscal.",
        ],
    ),
    "en/warehouse/forklift-safety.pdf": (
        "Forklift Safety Rules for Partner Warehouses",
        [
            "Fictitious document of Kestrel Analytics' logistics partners program.",
            "Load: partner warehouse forklifts carry at most 1,800 kg. Never lift a load above the rated capacity "
            "shown on the data plate.",
            "Speed: the speed limit inside partner warehouses is 8 km/h.",
            "Seat belts must be worn at all times. Passengers are never allowed on a forklift.",
            "Operators must renew their forklift license every 3 years.",
        ],
    ),
    "en/hr/corporate-card-policy.pdf": (
        "Corporate Card Policy",
        [
            "Fictitious document of Kestrel Analytics.",
            "Each corporate card has a monthly limit of USD 5,000. Limits above that need the approval of the CFO.",
            "Receipts must be uploaded to the expense tool within 10 days of the purchase. "
            "Cards with receipts missing for more than 30 days are frozen.",
            "The corporate card may not be used for personal expenses, cash withdrawals or gift cards.",
        ],
    ),
}


def main() -> None:
    styles = getSampleStyleSheet()
    for rel, (title, paragraphs) in DOCS.items():
        path = CORPUS / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        doc = SimpleDocTemplate(str(path), pagesize=A4, title=title, author="", invariant=1)
        story = [Paragraph(title, styles["Title"]), Spacer(1, 12)]
        for text in paragraphs:
            story += [Paragraph(text, styles["BodyText"]), Spacer(1, 8)]
        doc.build(story)
        print(f"wrote {rel}")


if __name__ == "__main__":
    main()
