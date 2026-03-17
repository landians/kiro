use crate::application::account::DefaultAccountService;
use crate::application::auth::AuthService;
use crate::application::health::HealthService;
use crate::application::login::{
    DefaultLoginService, GoogleAuthorizationRequest, GoogleLoginCallbackCommand, GoogleLoginResult,
    LoginServiceError,
};
use crate::application::user_identity::DefaultUserIdentityService;
use crate::domain::account::User;
use crate::domain::repository::user::UserRepositoryError;
#[cfg(test)]
use crate::{application::account::TestAccountService, application::login::TestLoginService};

pub mod account;
pub mod auth;
pub mod health;
pub mod login;
pub mod user_identity;

#[derive(Clone)]
pub struct AppServices {
    account: Option<DefaultAccountService>,
    pub auth: AuthService,
    pub health: HealthService,
    login: Option<DefaultLoginService>,
    #[allow(dead_code)]
    pub user_identity: Option<DefaultUserIdentityService>,
    #[cfg(test)]
    test_account: Option<TestAccountService>,
    #[cfg(test)]
    test_login: Option<TestLoginService>,
}

impl AppServices {
    pub fn new(
        account: Option<DefaultAccountService>,
        auth: AuthService,
        health: HealthService,
        login: Option<DefaultLoginService>,
        user_identity: Option<DefaultUserIdentityService>,
    ) -> Self {
        Self {
            account,
            auth,
            health,
            login,
            user_identity,
            #[cfg(test)]
            test_account: None,
            #[cfg(test)]
            test_login: None,
        }
    }

    pub fn account_service(&self) -> Option<AccountServiceRef<'_>> {
        if let Some(account_service) = self.account.as_ref() {
            return Some(AccountServiceRef::Default(account_service));
        }

        #[cfg(test)]
        if let Some(account_service) = self.test_account.as_ref() {
            return Some(AccountServiceRef::Test(account_service));
        }

        None
    }

    pub fn login_service(&self) -> Option<LoginServiceRef<'_>> {
        if let Some(login_service) = self.login.as_ref() {
            return Some(LoginServiceRef::Default(login_service));
        }

        #[cfg(test)]
        if let Some(login_service) = self.test_login.as_ref() {
            return Some(LoginServiceRef::Test(login_service));
        }

        None
    }

    #[cfg(test)]
    pub fn with_test_account(mut self, account: TestAccountService) -> Self {
        self.test_account = Some(account);
        self
    }

    #[cfg(test)]
    pub fn with_test_login(mut self, login: TestLoginService) -> Self {
        self.test_login = Some(login);
        self
    }
}

pub enum AccountServiceRef<'a> {
    Default(&'a DefaultAccountService),
    #[cfg(test)]
    Test(&'a TestAccountService),
}

impl AccountServiceRef<'_> {
    pub async fn find_user_by_user_code(
        &self,
        user_code: &str,
    ) -> Result<Option<User>, UserRepositoryError> {
        match self {
            Self::Default(account_service) => {
                account_service.find_user_by_user_code(user_code).await
            }
            #[cfg(test)]
            Self::Test(account_service) => account_service.find_user_by_user_code(user_code).await,
        }
    }
}

pub enum LoginServiceRef<'a> {
    Default(&'a DefaultLoginService),
    #[cfg(test)]
    Test(&'a TestLoginService),
}

impl LoginServiceRef<'_> {
    pub fn build_google_authorization_request(
        &self,
    ) -> Result<GoogleAuthorizationRequest, LoginServiceError> {
        match self {
            Self::Default(login_service) => login_service.build_google_authorization_request(),
            #[cfg(test)]
            Self::Test(login_service) => login_service.build_google_authorization_request(),
        }
    }

    pub async fn complete_google_login(
        &self,
        command: GoogleLoginCallbackCommand,
    ) -> Result<GoogleLoginResult, LoginServiceError> {
        match self {
            Self::Default(login_service) => login_service.complete_google_login(command).await,
            #[cfg(test)]
            Self::Test(login_service) => login_service.complete_google_login(command).await,
        }
    }
}
