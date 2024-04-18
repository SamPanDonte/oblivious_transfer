# Oblivious transfer

## Wykorzystane algorytmy:

- Grupa: P-256
- Funkcja derywacji klucza: sha256
- Algorytm szyfrowania symetrycznego: AES256

## Grupa: P-256

- Krzywe eliptyczne o 256 bitowym rozmiarze
- Zalecane przez (NIST)[https://csrc.nist.gov/csrc/media/events/workshop-on-elliptic-curve-cryptography-standards/documents/papers/session6-adalier-mehmet.pdf]
- Gwarantuje odporność na klasyczne i kwantowe ataki

## Funkcja derywacji klucza: SHA256

- Secure Hash Algorithm 256-bit - funkcja skrótu o rozmiarze wyjścia 256 bitów
- Odporna na kolizje i ataki
- Wykorzystywana jako standard np w sieci Bitcoin (jest dokładnie sprawdzona)

## Szyfrowanie symetryczne: AES256

- AES z kluczem o rozmiarze 256-bitów
- Sprzętowe wsparcie
- Przyjęty przez (NIST)[https://csrc.nist.gov/files/pubs/fips/197/final/docs/fips-197.pdf] jako standard od 2001 roku
- Powrzechnie używany do szyfrowania symetrycznego
- Tryb CBC zabezpiecza przed atakami i jest najczęściej wykorzystywanym trybem szyfrowania
