use leansig::signature::SignatureScheme;
use rand::rng;

type LeanSigScheme = leansig::signature::generalized_xmss::instantiations_poseidon_top_level::lifetime_2_to_the_32::hashing_optimized::SIGTopLevelTargetSumLifetime32Dim64Base8;

pub type PrivateKey = <LeanSigScheme as SignatureScheme>::SecretKey;

pub type PublicKey = <LeanSigScheme as SignatureScheme>::PublicKey;

pub fn generate_keypair(
    activation_epoch: usize,
    num_active_epochs: usize,
) -> (PublicKey, PrivateKey) {
    LeanSigScheme::key_gen(&mut rng(), activation_epoch, num_active_epochs)
}
