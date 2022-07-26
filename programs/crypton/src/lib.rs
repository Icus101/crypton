use anchor_lang::{prelude::*, solana_program::{system_instruction, program::{invoke}, native_token::LAMPORTS_PER_SOL}};
use anchor_spl::token::{ Mint, Token, TokenAccount, MintTo, Transfer, Burn};
use std::mem::size_of;
pub const ACCOUNT_SEED: &str = "acc-staking-pool";
pub const MINT_SEED: &str = "mint-staking-pool";
pub const WALLET_SEED: &str = "wallet";


declare_id!("FhrLHiXwBVmXGDLSF5hQVxmsn1bL1bEgMyT5nrSenyd5");
const MAX_DESCRIPTION_LEN: usize = 200;
const FEE_LAMPORTS: u64 = 2_000_000; // 0.002 SOL

#[program]
pub mod crypton {


    use anchor_spl::token;

    use super::*;

    pub fn initialize_fundraising(ctx: Context<StartFundraiser>,description:String) -> Result<()> {
        //require!(target > 0,CrowdFundError::InvalidTarget);
        let fundraiser = &mut ctx.accounts.fundraiser_state;
        fundraiser.fund_starter = ctx.accounts.fund_starter.key();
        require!(description.chars().count()<= MAX_DESCRIPTION_LEN,
        CrowdFundError::DescriptionTooLong);
        fundraiser.description = description;
        //fundraiser.target =target;
        fundraiser.balance = 0;
        fundraiser.status = Status::DonationsOpen.to_u8();
        fundraiser.donor_addr = Vec::new();//create a new list of donors
        let vault_add = ctx.accounts.vault.key.clone();
        fundraiser.vault_addr = vault_add;
        Ok(())
    }

    pub fn donate(ctx:Context<DonateSol>,amount_sol:u64,bump:u8) -> Result<()> {
        let current_status = Status::from(ctx.accounts.fundraiser_state.status)?;
        if current_status == Status::DonationsClosed || current_status == Status::CampaignEnded {
            msg!("This fundraising campaign is closed to Donations");
            return Err(CrowdFundError::ClosedToDonations.into());
        }
        if current_status != Status::DonationsOpen {
            msg!("Invalid status");
            return Err(CrowdFundError::InvalidStatus.into());
        }

        // //donate money to the campaign
         ctx.accounts.donate_money(amount_sol)?;
         msg!("{} lamports donated to the Campaign",amount_sol);

        // // transfer fee
         ctx.accounts.transfer_fee()?;

        //update the state of the account
        let fundraiser_state = &mut ctx.accounts.fundraiser_state;
        let donators= ctx.accounts.donor.key;
        fundraiser_state.balance += amount_sol;
        fundraiser_state.donor_addr.push(*donators);

        //send tokens to the referral
        let chrt_amt = amount_sol * 101;
        let seeds = &[MINT_SEED.as_bytes(), &[bump]];
        let signer = [&seeds[..]];
        token::mint_to(ctx.accounts.reward_refferals().with_signer(&signer),chrt_amt)?;

        // save users data
        let donors_acct = &mut ctx.accounts.donor_acc;
        donors_acct.amount_donated += amount_sol;


        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>,amount:u64) -> Result<()> {
        let fundraiser_state = &mut ctx.accounts.fundraiser_state;
        if Status::from(fundraiser_state.status)? != Status::CampaignEnded {
            fundraiser_state.status = Status::CampaignEnded.to_u8();
        }
       
         **ctx.accounts.vault.try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.destination.try_borrow_mut_lamports()? += amount;

        Ok(())
    }

    pub fn stop_campaign(ctx:Context<Stop>,amount_chrt : u64) -> Result<()>{
        token::burn(ctx.accounts.close_campaign(), amount_chrt)?;
        let state = &mut ctx.accounts.fundraiser_state;
        state.status = Status::DonationsClosed.to_u8();
        Ok(())
    }

    pub fn donate_chrt(ctx:Context<DonateChrt>,amount:u64) -> Result<()> {
        let current_status = Status::from(ctx.accounts.fundraiser_state.status)?;
        if current_status == Status::DonationsClosed || current_status == Status::CampaignEnded {
            msg!("This fundraising campaign is closed to Donations");
            return Err(CrowdFundError::ClosedToDonations.into());
        }
        if current_status != Status::DonationsOpen {
            msg!("Invalid status");
            return Err(CrowdFundError::InvalidStatus.into());
        }
        token::transfer(ctx.accounts.donate_chrt(), amount)?;
        Ok(())
    }


}

#[derive(Accounts)]
pub struct StartFundraiser <'info> {
    #[account(
        init,
        seeds = [ACCOUNT_SEED.as_ref()],
        bump,
        payer = fund_starter,
        space = 3000
    )]
    fundraiser_state : Account<'info,Fundraising>,
    ///CHECK :safe,just testing
    #[account(
        init,
        seeds = [fund_starter.key().as_ref()],
        bump,
        payer = fund_starter,
        space = 1000
    )]
    vault : AccountInfo<'info>,
    #[account(mut)]
    fund_starter : Signer<'info>,
    system_program : Program<'info,System>,
}

#[derive(Accounts)]
pub struct DonateSol<'info> {
    #[account(
        mut,
        seeds = [ACCOUNT_SEED.as_ref()],
        bump ,
        has_one = fund_starter,
    )] 
    fundraiser_state : Account<'info,Fundraising>,
    /// CHECK : safe
    #[account(
        mut,
        seeds = [fund_starter.key().as_ref()],
        bump ,
    )] 
    vault : AccountInfo<'info>,
    #[account(mut)]
    donor : Signer<'info>,
    ///CHECK: We only transfer commision to this account
    #[account(
        mut
    )]
    fee_vault : AccountInfo<'info>,
    ///CHECK : safe,since we do not read or write from this account
    fund_starter : AccountInfo<'info>,
    system_program: Program<'info,System>,
    ///CHECK: just testing
    #[account(
        seeds = [MINT_SEED.as_bytes()],
        bump,
    )]
    chrt_mint_authority:UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = referchrt_token_account.mint == chrt_mint.key()
    )]
    referchrt_token_account : Account<'info,TokenAccount>,
    #[account(mut)]
    chrt_mint : Account<'info,Mint>,
    token_program : Program<'info,Token>,
    #[account(
        init_if_needed,
        seeds = [donor.key().as_ref()],
        bump,
        payer= donor,
        space = 8 + size_of::<DonorsAcc>()
    )]
    donor_acc : Account<'info,DonorsAcc>
}
impl<'info> DonateSol<'info>{
    fn donate_money(&self,amount:u64) -> Result<()> {
        invoke(
            &system_instruction::transfer(
                self.donor.key,
                &self.vault.key(),
                amount * LAMPORTS_PER_SOL,
            ),
            &[
                self.donor.to_account_info(),
                self.vault.to_account_info(),
                self.system_program.to_account_info(),
            ],
        )?;
        Ok(())
        
    }
    fn transfer_fee(&self) -> Result<()>{
        invoke(
            &system_instruction::transfer(
                self.donor.key,
                &self.fee_vault.key(),
                FEE_LAMPORTS,
            ),
            &[
                self.donor.to_account_info(),
                self.fee_vault.to_account_info(),
                self.system_program.to_account_info()
            ],
        )?;
        Ok(())
    }
    
    fn reward_refferals(&self) -> CpiContext<'_,'_,'_,'info,MintTo<'info>>{
        let cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
             MintTo {  
                    mint: self.chrt_mint.to_account_info(),
                    to:self.referchrt_token_account.to_account_info(),
                    authority: self.chrt_mint_authority.to_account_info(),
                }
        );
        cpi_ctx
    }
}

#[derive(Accounts)]
pub struct Withdraw <'info> {
    #[account(
        mut,
        seeds = [ACCOUNT_SEED.as_ref()],
        bump ,
        has_one = fund_starter,
    )] 
    fundraiser_state : Account<'info,Fundraising>,
    /// CHECK:
    #[account(
        mut,
        seeds =[fund_starter.key().as_ref()],
        bump 
    )] 
    vault : AccountInfo<'info>,
    
    fund_starter : Signer<'info>,
    ///CHECK:
    #[account(mut)]
    destination : AccountInfo<'info>,
    system_program : Program<'info,System>,
}

#[derive(Accounts)]
pub struct DonateChrt <'info> {
    #[account(
        mut,
        seeds = [ACCOUNT_SEED.as_ref()],
        bump ,
        has_one = fund_starter,
    )]
    fundraiser_state : Account<'info,Fundraising>,
    #[account(
        mut,
        seeds=[b"funding-wallet".as_ref(), fund_starter.key().as_ref()],
        bump
    )]
    receiving_wallet: Account<'info, TokenAccount>,
    chrt_mint : Account<'info,Mint>,
    ///CHECK:
    fund_starter : AccountInfo<'info>,
    #[account(
        mut,
        constraint=donator_wallet.mint == chrt_mint.key(),
        constraint=donator_wallet.owner == donator.key()
    )]
    donator_wallet: Account<'info, TokenAccount>,
    #[account(mut)]
    donator: Signer<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}
impl<'info> DonateChrt<'info>{
    fn donate_chrt(&self) -> CpiContext<'_,'_,'_,'info,Transfer<'info>>{
        CpiContext::new(
            self.token_program.to_account_info(),
             Transfer{
                from: self.donator_wallet.to_account_info(),
                to: self.receiving_wallet.to_account_info(),
                authority: self.donator.to_account_info(),
             },
            )
    }
}
#[derive(Accounts)]
pub struct Stop<'info>{
    #[account(
        mut,
        seeds = [ACCOUNT_SEED.as_ref()],
        bump ,
        has_one = fund_starter,
    )]
    fundraiser_state : Account<'info,Fundraising>,
    #[account(mut)]
    fund_starter: AccountInfo<'info>,
    #[account(mut)]
    chrt_mint : Account<'info,Mint>,
    #[account(mut)]
    fund_starter_token_account : Account<'info,TokenAccount>,
    token_authority : Signer<'info>,
    token_program: Program<'info, Token>,
}
impl<'info> Stop <'info> {
    fn close_campaign(&self) -> CpiContext<'_,'_,'_,'info,Burn<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint:self.chrt_mint.to_account_info(), 
                from: self.fund_starter_token_account.to_account_info(),
                authority:self.token_authority.to_account_info()
            },
        )
    }
}



#[account]
pub struct Fundraising {
    // The admin of the fundraiser
    fund_starter : Pubkey,

    // the description of the fundraising
    description : String,

    // Amount donated
    balance : u64,
    status : u8,
    donor_addr : Vec<Pubkey>,
    vault_addr : Pubkey,
}

#[account]
pub struct DonorsAcc {
    amount_donated : u64,
    owner : Pubkey
}
#[derive(Clone,Copy,PartialEq,AnchorDeserialize,AnchorSerialize)]
pub enum Status {
    DonationsOpen,
    DonationsClosed,
    CampaignEnded
}

impl Status {
    fn from(val:u8) ->std::result::Result<Status,CrowdFundError>{
        match val {
            1 => Ok(Status::DonationsOpen),
            2 => Ok(Status::DonationsClosed),
            3 => Ok(Status::CampaignEnded),
            invalid_number => {
                msg!("Invalid state : {}",invalid_number);
                Err(CrowdFundError::InvalidStatus)
            }
        }
    }

    fn to_u8(&self) -> u8 {
        match self {
            Status::DonationsOpen =>1,
            Status::DonationsClosed => 2,
            Status::CampaignEnded => 3,

        }
    }
}


#[error_code]
pub enum CrowdFundError{
    #[msg("Target set for fundraising must be greater than Zero")]
    InvalidTarget,
    #[msg("Maxed out space for fund-raiser description")]
    DescriptionTooLong,
    #[msg("Invalid fundraiser status")]
    InvalidStatus,
    #[msg("You tried to donate to a closed fundraiser")]
    ClosedToDonations,
    #[msg("State balance does not correlate with wallet balance")]
    ErroneousBalance,

}
