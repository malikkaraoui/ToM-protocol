import XCTest
@testable import TomProtocolKit

/// Régression build 86→96 : `tom_node_receive_messages` renvoie le JSON de
/// `DeliveredMessageFFI` (Rust), qui ne contient PAS les champs sortants
/// ajoutés à `TomMessage` en #40 (`to`, `isAuto`, `deliveryStatus`). Le
/// `init(from:)` synthétisé exigeait la clé `isAuto` (non-optionnelle) →
/// `keyNotFound` → chaque batch reçu était jeté en silence → messages_recus=0
/// sur toute la flotte. Ces tests verrouillent le contrat de décodage.
final class TomMessageDecodingTests: XCTestCase {
    /// JSON EXACT tel que produit par le FFI (DeliveredMessageFFI, serde
    /// snake_case, sans champs sortants). NE PAS le "moderniser" : c'est le
    /// contrat wire réel.
    func testDecodeFFIIncomingBatch() throws {
        let json = #"""
        [{"from":"11d5bb113bb0b6c24819a7b2ad4396d533d2e5b8c23b622b51eeaa83b3d2d759","payload":"QUJD","envelope_id":"env-1","timestamp":1784260000000,"signature_valid":true,"was_encrypted":true}]
        """#
        let msgs = try JSONDecoder().decode([TomMessage].self, from: Data(json.utf8))
        XCTAssertEqual(msgs.count, 1)
        let m = try XCTUnwrap(msgs.first)
        XCTAssertEqual(m.id, "env-1")
        XCTAssertEqual(m.payloadData, Data("ABC".utf8))
        XCTAssertTrue(m.signatureValid)
        XCTAssertTrue(m.wasEncrypted)
        // Champs sortants absents du JSON FFI → valeurs par défaut, pas d'erreur.
        XCTAssertNil(m.to)
        XCTAssertFalse(m.isAuto)
        XCTAssertNil(m.deliveryStatus)
        XCTAssertFalse(m.isOutgoing)
    }

    /// Round-trip local (copies/persistance UI) : les champs sortants encodés
    /// par l'app survivent au décodage.
    func testRoundTripOutgoingFields() throws {
        var m = TomMessage(
            id: "env-2",
            from: "expediteur",
            payload: "QUJD",
            timestamp: 1,
            signatureValid: true,
            wasEncrypted: false
        )
        m.to = "destinataire"
        m.isAuto = true
        m.deliveryStatus = .delivered

        let data = try JSONEncoder().encode(m)
        let back = try JSONDecoder().decode(TomMessage.self, from: data)
        XCTAssertEqual(back.to, "destinataire")
        XCTAssertTrue(back.isAuto)
        XCTAssertEqual(back.deliveryStatus, .delivered)
        XCTAssertTrue(back.isOutgoing)
    }
}
