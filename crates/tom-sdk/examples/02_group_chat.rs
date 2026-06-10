//! Exemple groupe : alice crée un groupe auto-hébergé, bob est invité,
//! accepte, et reçoit un message de groupe.
//!
//! ```bash
//! cargo run -p tom-sdk --example 02_group_chat
//! ```

use tom_sdk::{Event, TomClientBuilder, TomSdkError};

#[tokio::main]
async fn main() -> Result<(), TomSdkError> {
    let mut alice = TomClientBuilder::new()
        .username("alice")
        .n0_discovery(false)
        .dht(false)
        .connect()
        .await?;
    let mut bob = TomClientBuilder::new()
        .username("bob")
        .n0_discovery(false)
        .dht(false)
        .connect()
        .await?;

    alice.add_peer_ticket(&bob.ticket()?).await?;
    bob.add_peer_ticket(&alice.ticket()?).await?;

    // Alice crée le groupe — elle en est le hub (auto-hébergé).
    alice
        .create_group("équipe", alice.id(), vec![bob.id()])
        .await?;

    // Bob attend l'invitation et l'accepte.
    while let Some(event) = bob.next_event().await {
        if let Event::GroupInviteReceived { invite } = event {
            println!("bob invité au groupe « {} »", invite.group_name);
            bob.accept_invite(invite.group_id).await?;
            break;
        }
    }

    // Alice attend que bob ait rejoint, puis poste.
    while let Some(event) = alice.next_event().await {
        if let Event::GroupMemberJoined { group_id, member } = event {
            println!("{} a rejoint le groupe", member.username);
            alice
                .send_group_message(group_id, "bienvenue dans l'équipe !")
                .await?;
            break;
        }
    }

    // Bob reçoit le message de groupe.
    while let Some(event) = bob.next_event().await {
        if let Event::GroupMessageReceived { message } = event {
            println!("[groupe] {} : {}", message.sender_username, message.text);
            break;
        }
    }

    alice.shutdown().await;
    bob.shutdown().await;
    Ok(())
}
